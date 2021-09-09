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
        link
        @click="navigate(item.tag)"
      >
      <strong>{{ item.tag }}</strong>&nbsp;<em>({{ item.count }})</em>
      </v-chip>
    </v-chip-group>
  </v-card>
</template>
<!-- 
The items can be a data table with filtering box. Columns can be clickable t provide sorting.<script>
Then plonk in pagination as well
      :search="type"

              :href="'http://localhost:8080/books/tags:&quot;%2F' + item.tag + '&quot;'"
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
                awaitingSearch: false,
                search:"",
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
        mounted () {
          this.$emit('categoriesInit');
          this.$axios.get('http://' + this.host + ':8000/api/counts/' + this.type + '?query=&countorder=true&limit=1000&start=' + ((this.page-1) * 1000))
              .then(response => ( 
                      this.items = response.data.payload,
                      this.count = response.data.count,
                      this.lastquery = response.data.query,
                      this.position = response.data.position
                      //this.zeroResult()
              )
          )
                
        },
        watch: {
          search: function () {
            if (!this.awaitingSearch) {
              setTimeout(() => {
                this.docountssearchof();
                this.awaitingSearch = false;
              }, 200); //200ms  delay
            }
            this.awaitingSearch = true;
          }
        },
        methods: {
            docountssearchof() {
                this.filtertext = this.search;
                this.errorMsg = null;
                this.$axios.get('http://' + this.host + ':8000/api/counts/' + this.type + '?query=' + this.search + '&countorder=true&limit=1000&start=' + ((this.page-1) * 1000))
                    .then(response => ( 
                            this.items = response.data.payload,
                            this.count = response.data.count,
                            this.lastquery = response.data.query,
                            this.position = response.data.position
                            //this.zeroResult()
                    )
                )
            },
            navigate(item) {
              this.$router.push({ name: 'books', params: { search:'tags:"/' + item + '"'} });
            }
        }

    }
</script>
