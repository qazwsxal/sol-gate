# sol-gate: A Freespace mod manager.

WORK IN PROGRESS

## Why?
As a minimum viable product, [Knossos](https://www.hard-light.net/forums/index.php?topic=94068.0) works well. However, there are several issues with it that have made continued work on it difficult. The complex build process of both old-knossos and new-knossos makes onboarding new developers troublesome, and limitations with fsnebula's `mods.json` file mean the big usability issues with old-knossos are fundamentally unfixable.
## User Features
### Minimise Check-for-updates time 
Smaller, conditional API calls instead of downloading a single ~270MB JSON of global state when checking for updates means sol-gate is ready to go ASAP.
### Speedy Updates
sol-gate tracks the contents of VP archives, so mod updates (both uploads and downloads) only consist of changed files, massively reducing download size.
### Locally Sourced Files
Many mods re-use models, textures, sounds and effects from each other. If another mod uses the same files, sol-gate will copy those instead of re-downloading them.
### FSNebula compatability 
Sol-gate is in very early development! The end goal here is a full replacement of the Knossos and FSNebula ecosystem, but sol-gate ships with the ability to query and install mods from FSNebula in the mean time.

## Modder Features
### As you meant it
If you include files with the same name in different packages (i.e. advanced graphics packs with higher-rez texture overrides), sol-gate won't complain, no need to trick it by hiding some in VPs and keeping others unbundled.

### FSO Builds and Tools
Freespace builds are treated as an entirely separate system to mods instead of being tacked-on as something like a dependency. Experimental builds are supported and need explicit opt-in from the user as a security feature. Modders can also exchange modelling and conversion tools and keep them automatically updated.

## Building - Devs only at the moment, sorry!
Make sure you have installed:

+ [Rust](https://www.rust-lang.org/)
+ [Node.js](https://nodejs.org/en/) - Windows users check [here](https://docs.microsoft.com/en-us/windows/dev-environment/javascript/nodejs-on-windows)

Clone repo and run in the root directory:
```shell
cargo run
```

This should automatically:
+ Pull in the npm dependencies,
+ Build the frontend
+ Pull in and build Rust dependencies
+ Build executable and embed frontend 
+ Run a debug version that will open in the browser.

Currently there's no button to trigger the API request to get the list of mods on FSNebula. Manually accessing `http://localhost:4000/api/fsn/mods/update` (link [here](http://localhost:4000/api/fsn/mods/update)) once the server is running will initiate getting the list. This command will take a while to return, and the page will load with the message `updated` once the modlist has been processed.

## Development
### Backend 
Development of backend is as can be expected, edit code and see if it works etc.

### Frontend 
Thankfully you don't need to know Rust for this! sol-gate's frontend is a react app that can be developed and iterated on as any other.

You can use the `react-scripts` dev server to speed up iteration.
In one terminal run:
```shell
cargo run
```
This will launch a sol-gate instance on `localhost:4000`. We can't edit the frontend of this instance, but we can proxy requests to it's backend.
In another terminal, cd into the `frontend` directory and run:
```shell
npm start
```
The npm development web server will boot up and open a page at `localhost:3000`. The page will will automatically reload when frontend files are saved. API requests are proxied to the running sol-gate instance. 

Any changes made to frontend code should be picked up by `cargo run` and/or `cargo build`. This will automatically recomple the frontend for deployment and embed the updated version in the new executable.


## Development FAQ

### Why Rust?
Any package/mod manager does a lot of file and network operations. These are both prone to failure. A problem that keeps popping up with Knossos is the assumption of the success of downloads and file operations due to the underlying libraries not correctly identifying failures. Rust's checked errors system forces us to handle or propagate these sorts of failures. Simply put, network and file operations fail a lot, and the control flow of Rust errors makes this easier to reason about and forces you to handle them.

### Why all this async/await nonsense?
This sort of program is heavily bound by file and network operations, and should be spending CPU cycles effectively on checksumming, database querying and de/compression rather than blocking on I/O. While Rust's asynchronous ecosystem still has some sharp edges (Pin, Unpin, Send/Sync across `.await` hell), it's happily being applied in global scale use cases. Using a cooperative multitasking runtime of many tasks across a fixed pool of threads also allows us to handle many short, concurrent operations (such as file downloading) without the overheads of thread creation (especially on our main target platform, Windows).  
### Why is this a web server that runs a page in the browser?
Cross-Platform GUI work isn't fun, and one of the major pain points for dev work on knossos was setting up Qt on Windows (this is also one of the reasons work on QtFRED is so intermittent). A web browser is a well specified, cross platform target that is strongly supported on all Freespace platforms. While Electron fills this role well, the Rust + Electron ecosystem doesn't seem mature at the moment. 

Secondly, as our file/network operations already lend us to using asynchronous runtimes, we might as well embrace this apprach and offload a lot of the work of concurrent GUI operations onto the Node ecosystem and browser runtime - a space where async is the norm.

Finally, the end goal of sol-gate is as a client and server program to replace *both* knossos and the fsnebula mod server. Architecting sol-gate as a web server from the beginning makes implementing this much easier. Functionality to download files from another sol-gate instance is already partially implemented, using the same mechanism as streaming a file into a new VP archive. However, the API calls necessary aren't wired up yet as there's no user authentication in place. 

In future, when sol-gate is in a reasonable state, the client might be migrated to something like [Tauri](https://tauri.app/) instead of Electron so that we can leverage system webviews rather than shipping a massive binary that's mostly chromium.
### Why are you using HTML+JS instead of a Rust frontend like [Yew](https://yew.rs/)?
Freespace is a small modding community with an even smaller pool of developers, there's a few devs who know Rust but they'd rather not do GUI work, and frontend developers don't usually know Rust. The simplicity of the build process means that (hopefully) someone with zero Rust experience is capable of contributing to frontend work.

