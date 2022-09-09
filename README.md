# sol-gate: A Freespace mod manager.

WORK IN PROGRESS

## Building
Make sure you have installed:

+ [Rust](https://www.rust-lang.org/)
+ npm - Windows users check [here](https://docs.microsoft.com/en-us/windows/dev-environment/javascript/nodejs-on-windows)

Clone repo and run in the root directory:
```shell
cargo run
```

This should automatically:
+ Pull in the npm dependencies,
+ Build the frontend
+ Pull in and build rust dependencies
+ Build executable and embed frontend 
+ Run a debug version that will open in the browser.

Currently there's no button to trigger the API request to get the list of mods on FSNebula. Manually accessing `http://localhost:4000/api/fsn/mods/update` (link [here](http://localhost:4000/api/fsn/mods/update)) once the server is running will initiate getting the list. This command will take a while to return, and the page will load with the message `updated` once the modlist has been processed.

## Development

Development of backend is as can be expected, edit code and see if it works etc.

### Frontend 
Frontend development can use the react-scripts to speed up iteration.
In one terminal run:
```shell
cargo run
```
In another terminal, cd into the `frontend` directory and run:
```shell
npm start
```
The development web server will boot up and open a page at `localhost:3000`. The page will will automatically reload when frontend files are saved. API requests are proxied to the running sol-gate instance, thus expected responses can be seen. 

Any changes made to the frontend should be automatically picked up by `cargo run` and `cargo build`, automatically recompiling the frontend and embedding it in the new executable.