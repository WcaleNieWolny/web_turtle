<script lang="ts">
  import Renderer from "./Renderer.svelte"
  import TurtleList from "./TurtleList.svelte";
  import TailwindCss from './TailwindCSS.svelte';

  let navbarDiv: HTMLDivElement;
</script>

<main>
  <TailwindCss/>
  <div class="main">
    <div bind:this={navbarDiv}>
      <TurtleList/>
    </div>
    <Renderer upperDiv={navbarDiv}/>
  </div>
</main>

<style>
  .main {
    display: flex;
    flex-direction: column;
    align-items: stretch;
    min-height: 100vh;
    margin: 0;
    width: 100%;
  }
</style>