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
        :key="item[pkfield]"
        color="#FFE0B2"
        link
        @click="navigate(item[pkfield])"
      >
      <strong>{{ item[pkfield] }}</strong>&nbsp;<em>({{ item.count }})</em>
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
          this.host = import.meta.env.VITE_SCHEME + "://" + window.location.hostname + import.meta.env.VITE_PORT ;
            return {
                type: null,
                pkfield: null,
                searchfield: null,
                page: 1,
                count: 0,
                position: 0,
                lastquery: null,
                host:this.host,
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
          this.init();
        },
        watch: {
          search: function () {
            if (!this.awaitingSearch) {
              setTimeout(() => {
                this.docountssearchof();
                this.awaitingSearch = false;
              }, 500); //200ms  delay
            }
            this.awaitingSearch = true;
          },
          $route() {
            this.init();
          }
        },
        methods: {
            init() {
              this.$emit('categoriesInit');
              this.type = this.$route.params.type;
              switch(this.type) {
                case "authors":
                  this.pkfield="creator";
                  this.searchfield="creator";
                  this.searchprepend="";
                  break;
                case "publishers":
                  this.pkfield="publisher";
                  this.searchfield="publisher";
                  this.searchprepend="";
                  break;
                case "tags":
                  this.pkfield="tag";
                  this.searchfield="tags";
                  this.searchprepend="/";
                  break;
                case "years":
                  this.pkfield="year";
              }
              this.$axios.get(this.host + '/api/counts/' + this.type + '?query=&countorder=true&limit=200&start=' + ((this.page-1) * 200))
                  .then(response => ( 
                          this.items = response.data.payload,
                          this.count = response.data.count,
                          this.lastquery = response.data.query,
                          this.position = response.data.position
                          //this.zeroResult()
                  )
              )
            },
            docountssearchof() {
                this.filtertext = this.search;
                this.errorMsg = null;
                this.$axios.get(this.host + '/api/counts/' + this.type + '?query=' + encodeURIComponent(this.search) + '&countorder=true&limit=1000&start=' + ((this.page-1) * 1000))
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
              this.$router.push({ name: 'books', params: { search:this.searchfield+':"' + this.searchprepend + item + '"'} });
            }
        }

    }
</script>
