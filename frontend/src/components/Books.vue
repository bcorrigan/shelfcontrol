<template>
  <v-container fluid fill-height >
    <v-row align-center >
      <v-col>
        <span v-html="errorMsg"></span>
        <v-list two-line >
          <v-list-subheader v-if="count" :key="count" inset id="resultList">
            {{ position + 1 }}-{{ Math.min(position + 20, count) }} of
            {{ count }} results for "{{ lastquery }}"
          </v-list-subheader>
          <span v-for="(book, index) in books" :key="book.id">
            <v-card style="word-break: normal">
              <v-card-title style="word-break: normal">
                <div>
                  <h2>
                    <span class="text-black">{{
                      book.title
                    }}</span>
                  </h2>
                  <span
                    style="cursor: pointer"
                    class="text-grey-darken-2"
                    @click="
                      clicksearch(
                        'creator:&quot;' + book.creator + '&quot;'
                      )
                    "
                  >
                    {{ book.creator }} </span
                  ><br />
              </div>
              </v-card-title>
              <!--finding out that this random property was washing out everything randomly here is why I hate UX dev-->
              <v-card-subtitle style="--v-medium-emphasis-opacity: 1;" >
                  <v-row>
                    <v-col cols="3" >
                      <v-img
                      max-width="200"
                      style="cursor: pointer; color: black;"
                      :src="this.host + '/img/' + book.id"
                      @click="
                        coverid = book.id;
                        coverdialog = true;
                      "
                    >
                      </v-img>
                    </v-col>
                    <v-col cols="9">
                      <span v-html="book.description" class="text-wrap; text-black"></span>
                    </v-col>
                  </v-row>
                  <br /><br />
                  <h5>
                    Published
                    <span v-if="book.moddate"
                      ><b>{{
                        new Date(
                          Date.parse(book.moddate)
                        ).toLocaleDateString()
                      }}</b></span
                    ><span v-if="book.publisher">
                      by
                      <b
                        ><span
                          style="cursor: pointer"
                          @click="
                            clicksearch(
                              'publisher:&quot;' + book.publisher + '&quot;'
                            )
                          "
                          class="text-amber-darken-4"
                          >{{ book.publisher }}.&nbsp;</span></b></span>
                          <span class="text-black">Size: {{ (book.filesize / 1048576).toFixed(2) }} Mb</span>
                  </h5>
              </v-card-subtitle>
              <v-card-actions>
                <v-row>
                  <v-col class="d-flex flex-wrap">
                  <span v-for="tag in book.subject" :key="tag">
                    <v-btn 
                      small
                      rounded
                      density="compact"
                      variant="text"
                      color="grey-darken-1"
                      @click="clicksearch('tags:&quot;/' + tag + '&quot;')"
                      class="text-lowercase"
                    >
                      {{ tag }}
                    </v-btn>
                  </span>
                </v-col>
                <v-responsive width="100%"></v-responsive>
                <v-col >
                <v-tooltip bottom>
                  <template v-slot:activator="{ props }">
                    <v-btn
                      text
                      color="black"
                      prepend-icon="mdi-download"
                      
                      v-bind="props"
                      @click="download(book)"
                      >
                      <template v-slot:prepend>
                        <v-icon color="amber-darken-3"></v-icon>
                      </template>
                      Download
                      </v-btn
                    >
                  </template>
                  <span>{{ (book.filesize / 1048576).toFixed(2) }} Mb</span>
                </v-tooltip>
                <v-btn
                  text
                  color="black"
                  prepend-icon="mdi-book-open-outline"
                  @click="
                    previewdialog = true;
                    readEpub(book);
                  "
                >
                  Preview
                  <template v-slot:prepend>
                    <v-icon color="amber-darken-3"></v-icon>
                  </template>
                </v-btn>
              </v-col>
            </v-row>
              </v-card-actions>
            </v-card>
            <v-divider v-if="index + 1 < books.length" :key="index"></v-divider>
          </span>
          <v-dialog
            v-model="previewdialog"
            fullscreen
            hide-overlay
            transition="dialog-bottom-transition"
          >
            <v-card style="position: relative">
              <v-row column fill-height>
                <v-toolbar color="primary">
                  <v-btn icon @click="previewdialog = false">
                    <v-icon>mdi-close</v-icon>
                  </v-btn>
                  <v-toolbar-title>Read book</v-toolbar-title>
                  <v-spacer></v-spacer>
                </v-toolbar>
                <!--<v-container class="fill-height">-->
                <div id="reader" style="height: 1000px; width: 100%" />
                <!--</v-container>-->
              </v-row>
            </v-card>
          </v-dialog>
          <v-pagination
            v-model="page"
            :length="Math.ceil(count / 20)"
            :total-visible="15"
            @input="next"
          >
          </v-pagination>
        </v-list>
      </v-col>
    </v-row>
    <v-dialog
      id="cdialog"
      v-model="coverdialog"
      @keydown.esc="coverdialog = false"
    >
      <v-img
        class="white--text"
        :src="this.host + '/img/' + coverid"
        @click="coverdialog = false"
        style="cursor: pointer"
        max-height="90vh"
        v-if="coverdialog"
      >
      </v-img>
    </v-dialog>
  </v-container>
</template>
<script>
    import { Book, Rendition } from '@parkdoeui/epubjs';
    import { watch } from 'vue';
    import { useRoute } from 'vue-router';
    export default {
            data: () => ({
      drawer: null,
      coverid: 0,
      readerKey: 0,
      books: null,
      isReady: false,
      count: 0,
      page: 1,
      position: 0,
      lastquery: null,
      coverdialog: null,
      previewdialog: false,
      errorMsg: null,
      searchtext: null,
      router:null,
      route: useRoute()
    }),
    props: {
      source: String
    },
    mounted () {
      this.hostbase = window.location.hostname;
      this.host = import.meta.env.VITE_SCHEME + '://' + this.hostbase + import.meta.env.VITE_PORT;
      var loadParams = this.$route.params.search;
      if(loadParams==undefined || loadParams.trim()=="") {
          loadParams='*';
      }
      this.$axios
        .get(this.host + '/api/search?query=' + encodeURIComponent(loadParams) + '&limit=20')
        .then(response => (this.books = response.data.payload , this.count = response.data.count, this.lastquery = response.data.query, this.position = response.data.position, this.$emit('bookSearch', response.data.query)));
    },
    watch: {
      $route(to, from) {
        //to and from are route objects
        this.searchtext=to.params.search;
        this.lastquery=from.params.search;
        this.dosearch();
      }
    },
    methods: {
      dosearchof (param) {
        this.searchtext = param;
        this.errorMsg=null;
        window.scrollTo(0,0);
        this.$axios
        .get(this.host + '/api/search?query=' + encodeURIComponent(param) + '&limit=20&start='+ ((this.page-1)*20))
        .then(response =>
          (this.books = response.data.payload,
          this.count = response.data.count,
          this.lastquery = response.data.query,
          this.position = response.data.position,
          this.$emit('bookSearch', response.data.query),
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
        this.$router.push({ name: 'books', params: { search:param } });
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
      readEpub(book) {
        //https://github.com/Janglee123/eplee/blob/db1af25ce0aafcccc9a2c3e7a9820bf8b6017b38/src/renderer/views/Reader.vue
        var reader = document.getElementById('reader');

        if(reader!=null) {
          while (reader.firstChild) {
            reader.removeChild(reader.firstChild);
          }
        }

        var epub = new Book(this.host + "/api/book/" + book.id + ".epub", { openAs: "epub" });
        this.rendition = new Rendition(epub, {
          manager: "continuous",
          flow: "scrolled",
          width: '100%',
          height: '100%',
        });

        this.rendition.on('rendered', (e, iframe) => {
          iframe.iframe.contentWindow.focus()
        });

        epub.ready
          .then(() => {
            this.rendition.attachTo(document.getElementById('reader'));
            this.rendition.display(1);
            this.rendition.ready = true;
          });
      },
      download(book) {
        this.$axios.get(this.host + "/api/book/" + book.id,
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
