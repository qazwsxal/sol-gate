<script>
	import '../style.css';
	import Footer from '$lib/Footer.svelte';
	import Navbar from '$lib/Navbar.svelte';
</script>

<div class="app">
	<div class="nav"><Navbar /></div>
	<div class="page"><slot /></div>

	<div class="foot"><Footer /></div>
</div>

<style>
	.app {
		height: 100vh;
		display: flex;
		flex-direction: column;
		flex-grow: 1;
		overflow: auto;
	}
	.nav,
	.foot {
		flex: 0 0 auto;
	}
	.page {
		align-items: center;
		display: flex;
		overflow: auto;
		flex-direction: column;
		flex: 1;
	}
</style>
