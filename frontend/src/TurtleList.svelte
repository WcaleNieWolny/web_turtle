<script lang="ts">
    import TurtleElement from './TurtleElement.svelte';
    import { type Turtle } from "./lib/turtle"

    let turtles: Turtle[] = [
        {uuid: "AAAAA", id: 0},
        {uuid: "AAAAB", id: 1},
        {uuid: "AAAAC", id: 2},
    ]
</script>

<div class="bg-stone-800 w-full h-16 mt-0 flex flex-row">
    {#each turtles as turtle}
        <TurtleElement turtle={turtle}/>
    {/each}
    <button class="h-11 w-11 self-end ml-auto mt-auto mb-auto mr-3 bg-cyan-500">
        <img src="./src/assets/update-icon.svg"/>    
    </button>
</div>