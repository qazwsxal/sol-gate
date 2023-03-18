<script>
	import { onMount } from "svelte";


	/**
	 * @type {string}
	 */
	export let mod_id;

    /**
	 * @type {any}
	 */
    let data = {
        title: "",
    }; 
	onMount(async () => {
		const res = await fetch('/api/mods/info/' + mod_id);
		data = await res.json();
	});
</script>

<div class=container>
  <div class="modtile" style="background-image: url({data.tile})">
    {data.title}
  </div>
</div>

<style>
    .container {
      padding: 0.5em;
    }
    .modtile {
        width: 150px;
        height: 225px;
        background-repeat: no-repeat;
      background-size: 100% 100%;
      background: #444
    }
  </style>
