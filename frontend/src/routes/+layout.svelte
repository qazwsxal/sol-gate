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
		height: 100%;
		display: flex;
		flex-direction: column;
	}
	.nav, .foot {
		flex: 0 0 auto;
	}
	.page {
		flex: 1;
		overflow: auto;
	}
</style>
