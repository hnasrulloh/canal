<script setup lang="ts">
import { onMounted, ref } from "vue"

onMounted(() => {
  products.value = [
    {
      id: "1000",
      code: "f230fh0g3",
      name: "Bamboo Watch",
      category: "Accessories",
      quantity: 24
    },
    {
      id: "11111",
      code: "1h23h12",
      name: "Clothe",
      category: "Apparel",
      quantity: 1
    },
    {
      id: "10982",
      code: "k12j3h",
      name: "Nike Shit",
      category: "Garbage",
      quantity: 0
    },
    {
      id: "12312",
      code: "0asd9a9sd",
      name: "Juice Jinggle",
      category: "Food",
      quantity: 0
    }
  ]
})

const products = ref()

const columns = [
  { field: "code", header: "Code" },
  { field: "name", header: "Name" },
  { field: "category", header: "Category" },
  { field: "quantity", header: "Quantity" }
]
</script>

<template>
  <div>
    <h1>Hi!</h1>

    <PrimeDataTable :value="products" tableStyle="min-width: 50rem">
      <PrimeColumn
        v-for="col of columns"
        :key="col.field"
        :field="col.field"
        :header="col.header"
      ></PrimeColumn>
    </PrimeDataTable>
  </div>
</template>
