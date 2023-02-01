use std::collections::HashMap;

use minilp::{ComparisonOp, LinearExpr, OptimizationDirection, Problem, Solution, Variable};

use crate::common::Source;

/// Calculate which sources are needed to optimally fetch the needed files
///
/// Given a map of source hash_ids to file hash_ids, and a map of how big each source is
///
///  
/// We can "solve" for the selection of sources to minimise network traffic.
/// for example, say we need files A, B and C. to update a mod.
/// and we know our sources are as such:
///
/// | Source | Files | Size  |
/// |--------|-------|-------|
/// | 1      | A, B  |  50MB |
/// | 2      | B, C  | 100MB |
/// | 3      | A, C  |  25MB |
/// | 4      | C     |  10MB |
///
///
/// Then we want to pick the sources that
///
/// 1. get all the hashes we need,
/// 2. minimise network usage.
///
/// This is a weighted set cover problem
/// that can be further transformed into a binary linear programming problem
/// i.e. (in non-canonical form):
/// + Minimise c^T x
/// + Subject to Ax >= b
/// + and x >= 0.
///
/// In our case:
/// + x is a binary (boolean) vector of "do we download this source"
/// + c is the sizes of our sources i.e. [50, 100, 25, 10],
/// + A is a matrix describing what hashes are present in which package.
/// ```ignore
/// A =  1  0  1  0
///      1  1  0  0
///      0  1  1  1
/// ```
/// Each column corresponds to one of the sources.
/// In this case, a solver would spit out `x = [1, 0, 0, 1]` with `c^T x = 60`.
/// Other solutions (using source 2 or 3) would require downloading more data.
///
/// This looks like you could just iterate through every possibility,
/// but in real terms this scales with O(2^n),
/// making most mods intractable.
/// Instead, we use an off-the-shelf linear programming solver with the following method:
///
/// 1. Ignore the integer constraint and solve the linear problem (linear relaxation)
/// 2. Add constraint to fix non-integer result.
/// 3. See if partial solution is better than our best fully integer-constrained solution.
/// 4. If not, stop exploring that constraint, if so, continue adding constraints and checking.
/// 5. Repeat until all branches are exhausted.
///
/// This is known as the *branch and bound* method. If we consider just "branching"
/// i.e. splitting our solution space up by fixing constraints to (0, 1) and exploring the tree,
/// then we're just doing an exhaustive search of every possible solution.
/// The "bounding" phase allows us to massively optimise this problem.
/// The value of the objective function for the partial solution is a lower bound
/// to *any* solution using the constraints of the partial solution.
/// If this lower bound is greater than our best solution,
/// then we know that all solutions using the constraints we've applied will be worse,
/// so we can skip exploring *any* solution that uses these constraints.
pub fn solve_files(sources: HashMap<Source, Vec<i64>>) -> Vec<Source> {
    // Based off of the TSP example from minilp:
    // https://github.com/ztlpn/minilp/blob/master/examples/tsp.rs
    // The TSP example boasts "pretty fast performance"
    // solving 150 points (~11,250 edges/variables) in ~3.7s on a 5800x (single threaded)
    // We're probably going to be faster than this because:
    // 1. we're usually dealing with less variables, it's 1 variable per source for us.
    // 2. we don't have to check for subtours and add constraints preventing them.
    // In fact, without these added subtour constraints, the TSP problem runs in <0.1s,
    // But who knows how much use that is as a benchmark.

    // this will be heavily commented as it's quite a complex process.

    // We instantate our problem.
    let mut problem = Problem::new(OptimizationDirection::Minimize);

    // This is a map from missing h_id to what source variables it can be found in.
    let mut constraints = HashMap::<i64, Vec<Variable>>::new();
    // Also keep track of how our "variable" labelling corresponds to our "sources"
    // needed as we can't rely on sources.iter() retaining order.
    let mut vars = Vec::<(Variable, Source)>::with_capacity(sources.len());

    for (source, h_ids) in sources.iter() {
        // We can download 0.0 or 1.0 of each source, so it's bounded in the range (0.0, 1.0)
        // this problem is called a *relaxation* - it relaxes the constraints on the variables being integers,
        // and instead allows them to take values in the range 0.0-1.0.
        let var = problem.add_var(source.size as f64, (0.0, 1.0));

        // While we're at it, we need to keep track of what sources each h_id can be found in.
        for h_id in h_ids {
            constraints.entry(*h_id).or_insert(Vec::new()).push(var)
        }

        // And how the "variables" correspond to the sources we want so that we can extract them later.
        vars.push((var, source.clone()))
    }
    // Now that we've generated: 1) our variables, 2) a map from h_id to what sources contain it,
    // We can add a system of constraints.
    for (h_id, source_vars) in constraints.into_iter() {
        // new linear expression (column of A)
        let mut sources_sum = LinearExpr::empty();
        // We're specifying which vars we can find the h_id in.
        for var in source_vars.into_iter() {
            // The coefficient is 1.0 as a source contains 1 * h_id
            sources_sum.add(var, 1.0)
        }
        // We don't keep track of which h_ids we're looking at as we want all of them to be >=1.0,
        // not finding even a single one is a failure.
        problem.add_constraint(sources_sum, ComparisonOp::Ge, 1.0)
    }

    let mut cur_solution = problem.solve().unwrap();
    // Now that we have a solution to the continuous problem, we can start our search for a integer solution.
    // This solution is still somewhere in the feasable region for the problem,
    // and by applying new constraints that force vars to be more integral, we can reach a solution.

    // We explore the space of possible variable values using the depth-first search.
    // Struct Step represents an item in the DFS stack. We will choose a variable and
    // try to fix its value to either 0 or 1. After we explore a branch where one value
    // is chosen, we return and try another value.
    struct Step {
        start_solution: Solution, // LP solution right before the step.
        var: Variable,
        start_val: u8,
        cur_val: Option<u8>,
    }

    // As we want to get to high-quality solutions as quickly as possible, choice of the next
    // variable to fix and its initial value is important. This is necessarily a heuristic choice
    // and we use a simple heuristic: the next variable is the "most fractional" one and its
    // initial value is the closest integer to the current solution value.

    // Returns None if the solution is integral.
    fn choose_branch_var(cur_solution: &Solution) -> Option<Variable> {
        let mut max_divergence = 0.0;
        let mut max_var = None;
        for (var, &val) in cur_solution {
            let divergence = f64::abs(val - val.round());
            if divergence > 1e-5 && divergence > max_divergence {
                max_divergence = divergence;
                max_var = Some(var);
            }
        }
        max_var
    }

    fn new_step(start_solution: Solution, var: Variable) -> Step {
        let start_val = if start_solution[var] < 0.5 { 0 } else { 1 };
        Step {
            start_solution,
            var,
            start_val,
            cur_val: None,
        }
    }

    // We will save the best solution that we encountered in the search so far and its cost.
    // After we finish the search this will be the optimal selection of sources.
    let mut best_cost = f64::INFINITY;
    let mut best_sources = None;

    // Initaialise our vector of steps
    let mut dfs_stack = if let Some(var) = choose_branch_var(&cur_solution) {
        vec![new_step(cur_solution, var)]
    } else {
        // this is our early out, if for some reason we've got all-integers already, there's no point doing any more.
        return sources_from_solution(&cur_solution, &vars);
    };
    // This can loop for a very long time, but not forever, the DFS will eventually terminate.
    // Calculating an upper bound is quite difficult and somewhat pointless as many searches will terminate early.
    for iter in 0.. {
        let cur_step = dfs_stack.last_mut().unwrap();

        // Choose the next value for the current variable.
        if let Some(ref mut val) = cur_step.cur_val {
            if *val == cur_step.start_val {
                *val = 1 - *val;
            } else {
                // We've expored all values for the current variable so we must backtrack to
                // the previous step. If the stack becomes empty then our search is done.
                dfs_stack.pop();
                if dfs_stack.is_empty() {
                    break;
                } else {
                    continue;
                }
            }
        } else {
            cur_step.cur_val = Some(cur_step.start_val);
        };

        let mut cur_solution = cur_step.start_solution.clone();
        if let Ok(new_solution) =
            cur_solution.fix_var(cur_step.var, cur_step.cur_val.unwrap() as f64)
        {
            cur_solution = new_solution;
        } else {
            // There is no feasible solution with the current variable constraints.
            // We must backtrack.
            continue;
        }

        let obj_val = cur_solution.objective();
        if obj_val > best_cost {
            // As the cost of any solution that we can find in the current branch is bound
            // from below by obj_val, it is pointless to explore this branch: we won't find
            // better solutions there.
            continue;
        }

        if let Some(var) = choose_branch_var(&cur_solution) {
            // Search deeper.
            dfs_stack.push(new_step(cur_solution, var));
        } else {
            // We've found an integral solution!
            if obj_val < best_cost {
                best_cost = obj_val;
                best_sources = Some(sources_from_solution(&cur_solution, &vars));
            }
        };
    }

    best_sources.unwrap()
}

// Convert a solution to a vector of the Sources we actually want to download.
// We don't keep track of which h_ids are present in each source as that's recoverable from the graph structure.
fn sources_from_solution(
    solution: &Solution,
    source_vars: &Vec<(Variable, Source)>,
) -> Vec<Source> {
    // At this point, the numbers will be pretty close to 0.0 or 1.0,
    // but make the cutoff 0.5 just in case.
    source_vars
        .iter()
        .filter_map(|(var, source)| (solution[*var] >= 0.5).then_some(source))
        .cloned()
        .collect()
}
