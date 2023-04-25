import { writable, type Writable } from 'svelte/store';

export interface Turtle {
    id: Number,
    uuid: String,
}

export enum MoveDirection {
  Forward = "forward",
  Backward = "backward",
  Left = "left",
  Right = "right",
}

export async function moveTurtle(turtle: Turtle, direction: MoveDirection) {
    const url = `${import.meta.env.VITE_BACKEND_URL}/turtle/${turtle.uuid}/move/`
    const body: string = direction.toString() 

    console.log(body)

    await fetch(url, {
        method: "PUT",
        body: body,
    })
}

export const turtleStore: Writable<Turtle> = writable(null);