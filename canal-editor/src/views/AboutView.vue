<script setup lang="ts">
import { invoke } from "@tauri-apps/api"
import { ref } from "vue"

const message = ref("")

invoke("message", { name: "Ferris" }).then((response) => {
  message.value = response as string
})
</script>

<template>
  <div class="about">
    <h1>This is an about page</h1>
    <h2>Message: {{ message }}</h2>
  </div>
</template>

<style>
@media (min-width: 1024px) {
  .about {
    min-height: 100vh;
    display: flex;
    align-items: center;
  }
}
</style>
