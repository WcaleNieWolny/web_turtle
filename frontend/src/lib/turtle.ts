import { writable, type Writable } from 'svelte/store';

export interface Turtle {
    id: Number,
    uuid: String,
}

export const turtleStore: Writable<Turtle> = writable(null);