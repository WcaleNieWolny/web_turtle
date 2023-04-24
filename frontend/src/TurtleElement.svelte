<script lang="ts">
    import { type Turtle, turtleStore} from "./lib/turtle"
    export let turtle: Turtle;

    let globalTurtle: Turtle = null;
    turtleStore.subscribe(val => {
        globalTurtle = val;
    })

    function onClick() {
        turtleStore.set(turtle)
    }
</script>

<div on:pointerdown={onClick}  class="bg-cyan-500 ml-1 mr-1 h-16 w-16 table align-middle text-center hover:bg-green-500 transition-colors delay-100 {globalTurtle !== null && globalTurtle.uuid === turtle.uuid ? 'green-border' : ''}">
    <p class="table-cell align-middle text-xl">{turtle.id}</p>
</div>

<style>
    .green-border
     {
        border-radius: 0.5rem;
        border-width: 5px;
        border-color: rgb(21 128 61)
    }
</style>