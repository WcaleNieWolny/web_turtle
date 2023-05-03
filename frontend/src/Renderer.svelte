<script lang="ts">
  import * as THREE from 'three';
  import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls';
  import { GLTFLoader } from 'three/examples/jsm/loaders/GLTFLoader';
  import { onMount } from 'svelte';
  import { turtleStore, type Turtle, MoveDirection, moveTurtle, directionToMoveDiff, directionToStartLocation, turtleRotateRight, turtleRotateLeft} from './lib/turtle';

  let canvas: HTMLCanvasElement
  export let upperDiv: HTMLDivElement

  const turtleModelUrl = new URL('./assets/turtle_model.glb', import.meta.url).href
  const scene = new THREE.Scene()
  let turtleModel: THREE.Group = undefined;

  let globalTurtle: Turtle = null;
  turtleStore.subscribe(val => {
      globalTurtle = val;
      if (turtleModel !== undefined) {
          let startLoc = directionToStartLocation(globalTurtle.rotation)
          turtleModel.position.x = startLoc.startX + globalTurtle.x
          turtleModel.position.z = startLoc.startZ + globalTurtle.z
          turtleModel.rotation.y = startLoc.rotY
      } 
  })

  let camera: THREE.PerspectiveCamera = null
  let renderer: THREE.WebGLRenderer = null;
 
   onMount(() => {
    scene.background = new THREE.Color(0x5b7cb6)

    renderer = new THREE.WebGLRenderer({
      canvas: canvas,
      antialias: true
    })
    renderer.setSize(canvas.clientWidth, canvas.clientHeight)

    camera = new THREE.PerspectiveCamera(75, canvas.width / canvas.height, 0.1, 1000 );

    renderer.outputEncoding = THREE.sRGBEncoding;

    const light = new THREE.AmbientLight( 0xffffff ); // soft white light
    scene.add( light );

    const geometry = new THREE.BoxGeometry( 1, 1, 1 );
    const material = new THREE.MeshBasicMaterial( { color: 0x56982e } );
    const cube = new THREE.Mesh( geometry, material );
    cube.position.set(0, 0, 0)
    scene.add( cube );


    const gridHelper = new THREE.GridHelper( 10, 10 );
    scene.add( gridHelper );

    const loader = new GLTFLoader();
    loader.load(
      // resource URL
      turtleModelUrl.toString(),
      // called when the resource is loaded
      function ( gltf ) {
        gltf.scene.position.set(0.5, 0.5, -0.5)
        turtleModel = gltf.scene
        turtleModel.rotation.y = -1.57
        //FORWARD = turtleModel.rotation.y = -1.57 (0.5, 0.5, -0.5)
        //BACK = 1.57 (-0.5, 0.5, 0.5)
        //turtleModel.rotation.y = Math.PI //Right 0.5, 0.5, 0.5 
        //iturtleModel.rotation.y = 0 //Left 0.5, 0.5, -0.5 
        scene.add( gltf.scene );
      },
      // called while loading is progressing
      function ( xhr ) {
        console.log( ( xhr.loaded / xhr.total * 100 ) + '% loaded' );
      },
      // called when loading has errors
      function ( ) {
        console.log( 'An error happened' );
      }
    );

    camera.position.z = 5;

    const controls = new OrbitControls( camera, renderer.domElement );
    controls.listenToKeyEvents( window ); // optional
    controls.enableDamping = true; // an animation loop is required when either damping or auto-rotation are enabled
    controls.dampingFactor = 0.05;
    controls.screenSpacePanning = false;
    controls.minDistance = 8;
    controls.maxDistance = 12;
    controls.maxPolarAngle = Math.PI / 2;

    function animate() {
      //cube.position.x += 0.1;
      if (turtleModel !== undefined) {
        controls.target = new THREE.Vector3(turtleModel.position.x, turtleModel.position.y, turtleModel.position.z);
        controls.update()
      }
      requestAnimationFrame( animate );

      //controls.update();
      renderer.render( scene, camera );
    }

    animate();
    });

    function onWindowsResize() {
      if (camera != null && renderer != null && canvas != null && upperDiv != null) {
        let width = window.innerWidth;
        let height = window.innerHeight - upperDiv.clientHeight
        renderer.setSize(width, height)

        camera.aspect = width / height
        camera.updateProjectionMatrix()
      }
    };

    let latestKeyPress = Date.now()

    async function onKeybordEvent(event: KeyboardEvent) {
      if (globalTurtle === null || turtleModel === null) {
        return
      }

      if (Date.now() - latestKeyPress < 500) {
        return
      }

      latestKeyPress = Date.now()
      let direction: MoveDirection = undefined;

      switch (event.key) {
        case "w": {
          direction = MoveDirection.Forward;
          break;
        }
        case "s": {
          direction = MoveDirection.Backward;
          break;
        }
        case "a": {
          direction = MoveDirection.Left;
          break
        }
        case "d": {
          direction =  MoveDirection.Right;
          break
        }
        default: {
          return;
        }
      }

      let result = await moveTurtle(globalTurtle, direction);
      if (!result.ok) {
        return
      }
      switch (direction) {
        case MoveDirection.Backward: 
        case MoveDirection.Forward: {
          let diff = directionToMoveDiff(direction, globalTurtle.rotation)
          turtleModel.position.x += diff.diffX
          turtleModel.position.y += diff.diffY
          turtleModel.position.z += diff.diffZ
          globalTurtle.x += diff.diffX
          globalTurtle.y += diff.diffY
          globalTurtle.z += diff.diffZ
          break
        }
        case MoveDirection.Right: {
          turtleRotateRight(globalTurtle)
          let startLoc = directionToStartLocation(globalTurtle.rotation)
          turtleModel.position.x = startLoc.startX + globalTurtle.x
          turtleModel.position.z = startLoc.startZ + globalTurtle.z
          turtleModel.rotation.y = startLoc.rotY
          break
        }
        case MoveDirection.Left: {
          turtleRotateLeft(globalTurtle)
          let startLoc = directionToStartLocation(globalTurtle.rotation)
          turtleModel.position.x = startLoc.startX + globalTurtle.x
          turtleModel.position.z = startLoc.startZ + globalTurtle.z
          turtleModel.rotation.y = startLoc.rotY
          break
        }
      }
    }
</script>


<canvas bind:this={canvas}></canvas>
<svelte:window on:resize={onWindowsResize} on:keydown={onKeybordEvent}/>

<style>
  canvas {
    display: flex;
    flex: 1
  }
</style>