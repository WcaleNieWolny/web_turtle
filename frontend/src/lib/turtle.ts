import { writable, type Writable } from 'svelte/store';
import type { Result } from './result';
import { z } from "zod";

export enum MoveDirection {
  Forward = "forward",
  Backward = "backward",
  Left = "left",
  Right = "right",
}

export const TurtleSchema = z.object({
    id: z.number(),
    uuid: z.string(),
    x: z.number(),
    y: z.number(),
    z: z.number(),
    rotation: z.nativeEnum(MoveDirection)
})

export type Turtle = z.infer<typeof TurtleSchema>

export function directionToMoveDiff(moveDirection: MoveDirection, turtleDirection: MoveDirection): { diffX: number, diffY: number, diffZ: number } {
    switch (turtleDirection) {
        case MoveDirection.Forward: {
            switch (moveDirection) {
                case MoveDirection.Forward: {
                    return {diffX: 1, diffY: 0, diffZ: 0}
                }
                case MoveDirection.Backward: {
                    return {diffX: -1, diffY: 0, diffZ: 0}
                }
                default: {
                    throw new Error("Invalid arguments")
                }
            }
        }
        case MoveDirection.Backward: {
            switch (moveDirection) {
                case MoveDirection.Forward: {
                    return {diffX: -1, diffY: 0, diffZ: 0}
                }
                case MoveDirection.Backward: {
                    return {diffX: 1, diffY: 0, diffZ: 0}
                }
                default: {
                    throw new Error("Invalid arguments")
                }
            }
        }
    }
}

export function directionToStartLocation(direction: MoveDirection): { startX: number, startY: number, startZ: number } {
    switch (direction) {
        case MoveDirection.Forward: {
            return { startX: 0.5, startY: 0.5, startZ: -0.5 };
        }
        case MoveDirection.Backward: {
            return { startX: -0.5, startY: 0.5, startZ: 0.5 };
        }
        //TODO: FIX THAT BELOW
        case MoveDirection.Left: {
            return { startX: -0.5, startY: 0.5, startZ: 0.5 };
        }
        case MoveDirection.Right: {
            return { startX: -0.5, startY: 0.5, startZ: 0.5 };
        }
    }
}

export async function moveTurtle(turtle: Turtle, direction: MoveDirection): Promise<Result<void, string>> {
    const url = `${import.meta.env.VITE_BACKEND_URL}/turtle/${turtle.uuid}/move/`
    const body: string = direction.toString() 

    let response = await fetch(url, {
        method: "PUT",
        body: body,
    })

    if (response.status == 200) {
        return { ok: true, value: undefined }
    } else {
        return { ok: false, error: await response.text() }
    }
}

export const turtleStore: Writable<Turtle> = writable(null);