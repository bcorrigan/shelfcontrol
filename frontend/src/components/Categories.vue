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
    <v-data-table
      :headers="headers"
      :items="items"
    ></v-data-table>
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
                this.$axios.get('http://' + this.host + ':8000/api/counts/' + this.type + '?query=' + this.search + '&countorder=true&limit=20&start=' + ((this.page-1) * 100))
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
