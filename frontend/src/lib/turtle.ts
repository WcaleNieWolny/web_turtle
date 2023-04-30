import { writable, type Writable } from 'svelte/store';
import type { Result } from './result';
import { z } from "zod";

export enum MoveDirection {
  Forward = "forward",
  Right = "right",
  Backward = "backward",
  Left = "left"
}

function directionToNumber(direction: MoveDirection) {
    switch (direction) {
        case MoveDirection.Forward: return 0
        case MoveDirection.Right: return 1
        case MoveDirection.Backward: return 2
        case MoveDirection.Left: return 3
    }
}

function numberToDirection(number: number): MoveDirection {
    switch (number) {
        case 0: return MoveDirection.Forward
        case 1: return MoveDirection.Right
        case 2: return MoveDirection.Backward
        case 3: return MoveDirection.Left
        default: throw Error("Invalid number to direction conversion") 
    }
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
        case MoveDirection.Left: {
            switch (moveDirection) {
                case MoveDirection.Forward: {
                    return {diffX: 0, diffY: 0, diffZ: -1}
                }
                case MoveDirection.Backward: {
                    return {diffX: 0, diffY: 0, diffZ: 1}
                }
                default: {
                    throw new Error("Invalid arguments")
                }
            }
        }
        case MoveDirection.Right: {
            switch (moveDirection) {
                case MoveDirection.Forward: {
                    return {diffX: 0, diffY: 0, diffZ: 1}
                }
                case MoveDirection.Backward: {
                    return {diffX: 0, diffY: 0, diffZ: -1}
                }
                default: {
                    throw new Error("Invalid arguments")
                }
            }
        }
    }
}

export function directionToStartLocation(direction: MoveDirection): { startX: number, startZ: number, rotY: number} {
    switch (direction) {
        case MoveDirection.Forward: {
            return { startX: 0.5, startZ: -0.5, rotY: -1.570796326794896619231321691639751442098584699687552910487472296 };
        }
        case MoveDirection.Backward: {
            return { startX: -0.5, startZ: 0.5, rotY: 1.570796326794896619231321691639751442098584699687552910487472296 };
        }
        case MoveDirection.Left: {
            return { startX: -0.5, startZ: -0.5, rotY: 0 };
        }
        case MoveDirection.Right: {
            return { startX: 0.5, startZ: 0.5, rotY: Math.PI };
        }
    }
}

export function turtleRotateRight(turtle: Turtle) {
    let enumInt = directionToNumber(turtle.rotation) 
    
    if (enumInt == 3) {
        enumInt = 0
    } else {
        enumInt += 1
    }

    turtle.rotation = numberToDirection(enumInt)
}

export function turtleRotateLeft(turtle: Turtle) {
    let enumInt = directionToNumber(turtle.rotation) 
    
    if (enumInt == 0) {
        enumInt = 3
    } else {
        enumInt -= 1
    }

    turtle.rotation = numberToDirection(enumInt) 
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