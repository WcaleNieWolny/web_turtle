<script lang="ts">
    interface turtle {
        uuid: String,
    }

    let turtles: turtle[] = [
        {uuid: "AAAAA"},
        {uuid: "AAAAA"},
        {uuid: "AAAAA"}
    ]
</script>

<div>
{#each turtles as turtle, i}
    <p>{i}</p>
{/each}
</div>

<style>
    p {
        margin: 1rem;
        font-size: 1.15rem;
    }
    div {
        background-color: #292524;
        width: 100%;
        margin-top: 0;
        display: flex;
        flex-direction: row;
    }
</style>