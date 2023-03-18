<script>
	import ModTile from "$lib/ModTile.svelte";
	import { onMount } from "svelte";
    /**
	 * @type {any[]}
	 */
    let mods = []; 
	onMount(async () => {
		const res = await fetch('/api/mods/avaliable');
		mods = await res.json();
	});
</script>



<div class="modtiles">
{#each mods as mod}
<ModTile mod_id={mod.id}/>
{/each}
</div>


<style>
    .modtiles {
	display: flex;
	flex-wrap: wrap;
	overflow: auto;
	justify-content: space-around;
    }
  </style>
