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
      color: white;
    }
    .modtile {
        width: 150px;
        height: 225px;
        background-repeat: no-repeat;
      background-size: 100% 100%;
    }
    .img {
    width: 150px;
      height: 255px;
      
    }
    ul {
      display: flex;
      margin: 0;
      list-style: none;
      font-size: 1em;
    }
    li {
      margin-right: 20px;
    }
    /* h1 {
      margin: 0;
      font-size: 1.3em;
      font-weight: normal;
    } */
    a {
      text-decoration: none;
      color: #aaa;
    }
    /* .logo {
      color: white;
    } */
    .active {
      color: white;
    }
  </style>
