<script lang="ts">
    import { writable, type Writable } from 'svelte/store';
    import TurtleElement from './TurtleElement.svelte';
    import { turtleStore, type Turtle } from "./lib/turtle"
    import { onMount } from 'svelte';

    let turtles: Writable<Turtle[]> = writable([]) 

    let globalTurtle: Turtle = null;
    turtleStore.subscribe(val => {
        globalTurtle = val;
    })

    async function fetchTurtles() {
        const url = `${import.meta.env.VITE_BACKEND_URL}/turtle/list/`
        const response = await fetch(url, {
            method: "GET"
        })
        const remoteTurtles: string[] = await response.json();
        const newTurtles: Turtle[] = []

        remoteTurtles.forEach((uuid, i) => {
            newTurtles.push({
                uuid: uuid,
                id: i
            })
        });

        //Unselect turtle
        if (globalTurtle != null && newTurtles.find(turtle => turtle.uuid === globalTurtle.uuid) === undefined) {
            turtleStore.set(null)
        }
        turtles.set(newTurtles)
    }

    onMount(async () => {
        await fetchTurtles()
    })
</script>

<div class="bg-stone-800 w-full h-16 mt-0 flex flex-row">
    {#each $turtles as turtle}
        <TurtleElement turtle={turtle}/>
    {/each}
    <button on:click={fetchTurtles} class="h-11 w-11 self-end ml-auto mt-auto mb-auto mr-3 bg-cyan-500 rounded-full border-[6px] border-cyan-500">
        <img src="./src/assets/update-icon.svg" alt="refresh button"/>    
    </button>
</div>