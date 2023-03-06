<script>
    import { page } from "$app/stores";
    import { onMount } from "svelte";
    const navs = [
      {
        title: "Play",
        href: "/",
      },
      {
        title: "Explore",
        href: "/explore",
      },
      {
        title: "FSNebula (legacy)",
        href: "/fsn",
      },
      {
        title: "Settings",
        href: "/settings",
      },
    ];

    $: url = $page.url.href;
    $: routeId = $page.url.pathname;
  </script>
  
  <nav>
    <div class="container">
      <ul>
        {#each navs as { title, href }}
          <li>
            <a
              {href}
              class:active={href === "/" ? routeId === "/" : url.includes(href)}
              {title}>{title}</a
            >
          </li>
        {/each}
      </ul>
    </div>
  </nav>
  
  <style>
    nav {
      padding: 0.5em;
      background-color: #343a40;
      color: white;
      align-content: center;
      display: f;
    }
    .container {
      display: flex;
      align-items: center;
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