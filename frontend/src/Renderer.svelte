<script lang="ts">
  import * as THREE from 'three';
  import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls';
  import { GLTFLoader } from 'three/examples/jsm/loaders/GLTFLoader';
  import { onMount } from 'svelte';

  let div: HTMLDivElement
  let canvas: HTMLCanvasElement

  const turtleModelUrl = new URL('./assets/turtle_model.glb', import.meta.url).href
  const scene = new THREE.Scene()

  let camera: THREE.PerspectiveCamera = null
  let renderer: THREE.WebGLRenderer = null;
  
  onMount(() => {
    scene.background = new THREE.Color(0x5b7cb6)

    renderer = new THREE.WebGLRenderer({
      canvas: canvas
    })
    canvas.width = div.clientWidth;
    canvas.height = div.clientHeight;
    renderer.setSize(canvas.width, canvas.height)

    camera = new THREE.PerspectiveCamera(75, window.innerWidth / window.innerHeight, 0.1, 1000 );
    camera.aspect = div.clientWidth / div.clientHeight;
    camera.updateProjectionMatrix()

    renderer.outputEncoding = THREE.sRGBEncoding;

    const light = new THREE.AmbientLight( 0xffffff ); // soft white light
    scene.add( light );

    const geometry = new THREE.BoxGeometry( 1, 1, 1 );
    const material = new THREE.MeshBasicMaterial( { color: 0x56982e } );
    const cube = new THREE.Mesh( geometry, material );
    cube.position.set(0, 0, 0)
    scene.add( cube );

    const loader = new GLTFLoader();
    loader.load(
      // resource URL
      turtleModelUrl.toString(),
      // called when the resource is loaded
      function ( gltf ) {
        gltf.scene.position.set(-0.5, 0.5, -0.5)
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
    controls.maxDistance = 15;
    controls.maxPolarAngle = Math.PI / 2;

    function animate() {
      //cube.position.x += 0.1;
      controls.target = new THREE.Vector3(cube.position.x, cube.position.y, cube.position.y);
      controls.update()
      requestAnimationFrame( animate );

      //controls.update();
      renderer.render( scene, camera );
    }

    animate();
    });

    function onWindowsResize() {
      if (camera != null && renderer != null && canvas != null && div != null) {
        canvas.width = div.clientWidth;
        canvas.height = div.clientHeight;
        renderer.setSize(canvas.width, canvas.height)
        camera.updateProjectionMatrix()
      }
    };

</script>


<div bind:this={div}>
  <canvas bind:this={canvas}></canvas>
</div>
<svelte:window on:resize={onWindowsResize}/>

<style>
  div {
    flex: 1;
    align-self: stretch;
    display: grid;
    grid-template-columns: 1fr;
    grid-template-rows: 1fr;
    gap: 0px 0px
  }
</style>