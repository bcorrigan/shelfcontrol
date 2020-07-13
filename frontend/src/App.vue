<template>
  <v-app id="shelfcontrol">
    <v-app-bar color="amber darken-1" clipped-left app>
      <v-app-bar-nav-icon @click="drawer = !drawer"></v-app-bar-nav-icon>
      <span class="title ml-3 mr-5">Shelf&nbsp;<span class="font-weight-light">Control</span></span>
      <v-text-field
      v-model="searchtext"
        solo-inverted
        flat
        hide-details
        label="Search"
        @change="dosearch"
        prepend-inner-icon="search"
      ></v-text-field>
      <v-spacer></v-spacer>
    </v-app-bar>
    <v-navigation-drawer
      v-model="drawer"
      class="grey lighten-2"
      clipped
      app
    >
      <v-spacer class="v-spacer">&nbsp;</v-spacer>
      <v-spacer class="v-spacer">&nbsp;</v-spacer>
      <v-spacer class="v-spacer">&nbsp;</v-spacer>
      <v-list
        dense
        class="grey lighten-2"
      >
        <template v-for="(item, i) in items">
          <v-layout
            v-if="item.heading"
            :key="i"
            row
            align-center
          >
            <v-flex xs6>
              <v-subheader v-if="item.heading">
                {{ item.heading }}
              </v-subheader>
            </v-flex>
            <!--<v-flex xs6 class="text-xs-right">
              <v-btn small>edit</v-btn>
            </v-flex>-->
          </v-layout>
          <v-divider
            v-else-if="item.divider"
            :key="i"
            dark
            class="my-3"
          ></v-divider>
          <v-list-item
            v-else
            :key="i"
          >
            <v-list-item-action>
              <v-icon>{{ item.icon }}</v-icon>
            </v-list-item-action>
            <v-list-item-content>
              <v-list-item-title class="dark-grey--text">
                {{ item.text }}
              </v-list-item-title>
            </v-list-item-content>
          </v-list-item>
        </template>
      </v-list>
    </v-navigation-drawer>
    <v-main>
      <v-container fluid fill-height class="grey lighten-4">
        <v-layout align-center>
          <v-flex>
            <span v-html="errorMsg"></span>
        <v-list two-line>
          <v-subheader
            v-if="count"
            :key="count"
            inset
            id="resultList"
          >
            {{ position + 1 }}-{{ Math.min(position+20,count) }} of {{ count }} results for "{{ lastquery }}"
          </v-subheader>
          <span v-for="(book, index) in books" :key="book.id">
            <v-card style="word-break: normal">
              <v-layout align-center>
                <v-flex xs5>
              <v-img
                class="white--text"
                height="400"
                contain
                style="cursor: pointer"
                :src="'http://localhost:8000/img/' + book.id"
                @click="coverid = book.id;  coverdialog = true">
              </v-img>
                  </v-flex>
                <v-flex xs7 d-flex>
                  <v-layout column>
                    <v-flex xs12>
              <v-card-title style="word-break: normal">
                <div>
                    <h2><span class="grey--text text--darken-3">{{ book.title }}</span></h2>
                    <span
                      style="cursor: pointer"
                      class="grey--text text--darken-2"
                      @click="clicksearch('creator:&quot;' + book.creator + '&quot;')"
                    >
                    {{book.creator}}
                  </span><br>
                  <div v-html="book.description" class="text-body-1"></div>
                  <br><br>
                  <h4><span
                        style="cursor: pointer"
                        @click="clicksearch('publisher:&quot;' + book.publisher + '&quot;')"
                        class="grey--text text--darken-3">{{book.publisher}}</span></h4>
                </div>
              </v-card-title>
                      </v-flex>
                              <v-card-actions>
                                <v-layout row wrap justify-left>
                                  <span v-for="tag in book.subject" :key="tag">
                                    <v-btn
                                      small
                                      rounded
                                      color="grey lighten-2"
                                      @click="clicksearch('tags:&quot;/' + tag + '&quot;')"
                                      class="text-lowercase"
                                      >
                                        {{ tag }}
                                    </v-btn> <!-- there must be some better way than this nbsp uglyness -->
                                    &nbsp;
                                    <v-spacer class="v-spacer">&nbsp;</v-spacer>
                              </span>
                              </v-layout>
                                <v-tooltip bottom>
                                  <template v-slot:activator="{ on }">
                                    <v-btn text color="orange" v-on="on" @click="download(book)">Download</v-btn>
                                  </template>
                                  <span>{{(book.filesize / 1048576).toFixed(2)}} Mb</span>
                                </v-tooltip>
                              </v-card-actions>
                    </v-layout>
                  </v-flex>
                </v-layout>
            </v-card>
              <v-divider
                v-if="index + 1 < books.length"
                :key="index"
              ></v-divider>
          </span>
          <v-pagination
             v-model="page"
             :length="Math.ceil(count / 20)"
             :total-visible="15"
             @input="next">
          </v-pagination>
        </v-list>
      </v-flex>
    </v-layout>
      </v-container>
      <v-dialog
        id="cdialog"
        v-model="coverdialog"
        @keydown.esc="coverdialog = false"
      >
        <v-img
          class="white--text"
          :src="'http://' + host + ':8000/img/' + coverid"
          @click="coverdialog=false"
          style="cursor: pointer"
          v-if="coverdialog"
        >
        </v-img>
      </v-dialog>
    </v-main>
  </v-app>
</template>

<script>
  export default {
    data: () => ({
      drawer: null,
      items: [
        { icon: 'book', text: 'All Books' },
        { divider: true },
        { heading: 'Browse' },
        { icon: 'face', text: 'Authors' },
        { icon: 'bookmark', text: 'Tags' },
        { divider: true },
        { icon: 'settings', text: 'Settings' },
        { icon: 'help', text: 'Help' },
      ],
      coverid: 0,
      books: null,
      count: 0,
      page: 1,
      position: 0,
      lastquery: null,
      coverdialog: null,
      errorMsg: null,
      searchtext: null,
      host:"localhost"
    }),
    props: {
      source: String
    },
    mounted () {
      this.$axios
        .get('http://' + this.host + ':8000/api/search?query=tolkien&limit=20')
        .then(response => (this.books = response.data.books , this.count = response.data.count, this.lastquery = response.data.query, this.position = response.data.position));
      this.host = window.location.hostname;
    },
    methods: {
      dosearchof (param) {
        this.searchtext = param;
        this.errorMsg=null;
        window.scrollTo(0,0);
        this.$axios
        .get('http://' + this.host + ':8000/api/search?query=' + param + '&limit=20&start='+ ((this.page-1)*20))
        .then(response =>
          (this.books = response.data.books,
          this.count = response.data.count,
          this.lastquery = response.data.query,
          this.position = response.data.position,
          this.zeroResult()
          )
        )
      },
      dosearch () {
        //change event sometimes lies - it is fired even when text is not changed since last time
        if(this.searchtext!=this.lastquery || this.page!=1) {
          this.page=1;
          this.dosearchof(this.searchtext);
        }
      },
      clicksearch (param) {
        this.page=1;
        this.dosearchof(param);
      },
      next (page) {
        this.page=page;
        this.dosearchof(this.lastquery);
      },
      zeroResult () {
        if(this.count==0) {
          this.errorMsg='<h3>No results for "<b>' + this.lastquery + '</b>"</h3><p/>&nbsp;<p/>&nbsp;<p/>&nbsp;<p/>&nbsp;';
        }
      },
      download(book) {
        this.$axios.get("http://" + this.host + ":8000/api/book/" + book.id,
        {
            responseType: 'arraybuffer',
            headers: {
                'Content-Type': 'application/json',
                'Accept': 'application/epub+zip'
            }
        })
        .then((response) => {
            const url = window.URL.createObjectURL(new Blob([response.data]));
            const link = document.createElement('a');
            link.href = url;
            link.setAttribute('download', book.creator + ' - ' + book.title + '.epub'); //or any other extension
            document.body.appendChild(link);
            link.click();
        })
        .catch((error) => console.log(error));
      }
    }
  }
</script>

<style lang="stylus">
  #keep
    .v-navigation-drawer__border
      display: none
    .v-spacer {
      font-size: 0.2em;
    }
</style>
