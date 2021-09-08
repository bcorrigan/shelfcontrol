<template>
  <v-card fill-height fluid>
    <v-card-title>
      <v-text-field
        v-model="search"
        append-icon="mdi-magnify"
        label="Filter"
        single-line
        hide-details
        @change="docountssearchof()"
      ></v-text-field>
    </v-card-title>

    <v-chip-group
      column
    >
      <v-chip
        v-for="item in items"
        :key="item.tag"
        color="#FFE0B2"
        :href="'http://localhost:8080/books/tags:&quot;%2F' + item.tag + '&quot;'"
      >
      <strong>{{ item.tag }}</strong>&nbsp;<em>({{ item.count }})</em>
      </v-chip>
    </v-chip-group>

<!--    <v-data-table
      :headers="headers"
      :items="items"
    >
        <template v-slot:[`item.tag`]="{ item }">
          <v-chip
            color="#FFE0B2"
            outline
            label
            link
            :href="'http://localhost:8080/books/tags:&quot;%2F' + item.tag + '&quot;'"
          >
            {{ item.tag }}
          </v-chip>
        </template>
    </v-data-table> -->
  </v-card>
</template>
<!-- 
The items can be a data table with filtering box. Columns can be clickable t provide sorting.<script>
Then plonk in pagination as well
      :search="type"
</script>

-->
<script>
    export default {
        data () {
            return {
                type: "tags",
                page: 1,
                count: 0,
                position: 0,
                lastquery: null,
                 host:"localhost",
                headers: [
                    {
                        text: "Tag",
                        align: "start",
                        value: "tag"
                    },
                    {
                        text: "Books",
                        value: "count"
                    }
                ],
                items: []
            }
        },
        methods: {
            docountssearchof() {
                this.filtertext = this.search;
                this.errorMsg = null;
                this.$axios.get('http://' + this.host + ':8000/api/counts/' + this.type + '?query=' + this.search + '&countorder=true&limit=100&start=' + ((this.page-1) * 100))
                    .then(response => ( 
                            this.items = response.data.payload,
                            this.count = response.data.count,
                            this.lastquery = response.data.query,
                            this.position = response.data.position
                            //this.zeroResult()
                    )
                )
            }
        }

    }
</script>
