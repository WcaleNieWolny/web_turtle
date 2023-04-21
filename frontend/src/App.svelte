<script lang="ts">
  import Renderer from "./Renderer.svelte"
</script>

<main>
  <div class="main">
    <div class="test">
      <p>
        Hello
      </p>
    </div>
    <Renderer/>
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

  .test {
    flex-shrink: 0;
  }
</style>