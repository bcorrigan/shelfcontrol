<template>
  <v-container fluid fill-height  >
    <v-row align-center >
      <v-col>
        <span v-html="errorMsg"></span>
        <v-data-iterator :items="books" :items-per-page="this.pageSize">
          <template v-slot:header="">
            <v-list-subheader v-if="count" :key="count" inset id="resultList">
            {{ position + 1 }}-{{ Math.min(position + this.pageSize, count) }} of
            {{ count }} results for "{{ lastquery }}"
          </v-list-subheader>
          </template>
          <template v-slot:default="{items}">
          <v-list-item v-for="(book, index) in items" :key="book.raw.id" :ref="'card-' + index" >
            <v-card style="word-break: normal">
              <v-card-title style="word-break: normal">
                <div :id="index" style="scroll-margin-top: 100px;">
                  <h2>
                    <span class="text-black">{{
                      book.raw.title
                    }}</span>
                  </h2>
                  <span
                    style="cursor: pointer"
                    class="text-grey-darken-2"
                    @click="
                      clicksearch(
                        'creator:&quot;' + book.raw.creator + '&quot;'
                      )
                    "
                  >
                    {{ book.raw.creator }} </span
                  ><br />
              </div>
              </v-card-title>
              <!--finding out that this random property was washing out everything randomly here is why I hate UX dev-->
              <v-card-subtitle style="white-space:break-spaces; --v-medium-emphasis-opacity: 1; " >
                  <v-row>
                    <v-col cols="3" >
                      <v-img
                      max-width="200"
                      style="cursor: pointer; color: black;"
                      :src="this.host + '/img/' + book.raw.id"
                      @click="
                        coverid = book.raw.id;
                        coverdialog = true;
                      "
                    >
                      </v-img>
                    </v-col>
                    <v-col cols="9" class="white-space: normal; word-break: break-word; text-wrap; text-black">
                      <span v-html="book.raw.description" class="white-space: normal; word-break: break-word; text-wrap; text-black"></span>
                    </v-col>
                  </v-row>
                  <br /><br />
                  <h5>
                    Published
                    <span v-if="book.raw.moddate"
                      ><b>{{
                        new Date(
                          Date.parse(book.raw.moddate)
                        ).toLocaleDateString()
                      }}</b></span
                    ><span v-if="book.raw.publisher">
                      by
                      <b
                        ><span
                          style="cursor: pointer"
                          @click="
                            clicksearch(
                              'publisher:&quot;' + book.raw.publisher + '&quot;'
                            )
                          "
                          class="text-amber-darken-4"
                          >{{ book.raw.publisher }}.&nbsp;</span></b></span>
                          <span class="text-black">Size: {{ (book.raw.filesize / 1048576).toFixed(2) }} Mb</span>
                  </h5>
              </v-card-subtitle>
              <v-card-actions>
                <v-row>
                  <v-col class="d-flex flex-wrap">
                  <span v-for="tag in book.raw.subject" :key="tag">
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
                      @click="download(book.raw)"
                      >
                      <template v-slot:prepend>
                        <v-icon color="amber-darken-3"></v-icon>
                      </template>
                      Download
                      </v-btn
                    >
                  </template>
                  <span>{{ (book.raw.filesize / 1048576).toFixed(2) }} Mb</span>
                </v-tooltip>
                <v-btn
                  text
                  color="black"
                  prepend-icon="mdi-book-open-outline"
                  @click="
                    previewdialog = true;
                    readEpub(book.raw);
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
            <v-divider v-if="index + 1 < items.length" :key="index"></v-divider>
          </v-list-item>
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
          </template>
          <template v-slot:footer="">
            <v-pagination
              v-model="page"
              :length="Math.ceil(count / this.pageSize)"
              :total-visible="15"
              @update:model-value="next"
            >
            </v-pagination>
        </template>
        </v-data-iterator>
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
    import { useAppBarHeight } from './useAppBarHeight';
    export default {
            data: () => ({
      drawer: null,
      coverid: 0,
      readerKey: 0,
      books: [],
      isReady: false,
      count: 0,
      page: 1,
      pageSize: import.meta.env.VITE_PAGE_SIZE,
      position: 0,
      lastquery: null,
      coverdialog: null,
      previewdialog: false,
      errorMsg: null,
      searchtext: null,
      router:null,
      route: useRoute(),
      selectedIndex: 0
    }),
    props: {
      source: String
    },
    setup() {
      const { appBarHeight } = useAppBarHeight();
      return { appBarHeight }
    },
    mounted () {
      this.host = import.meta.env.VITE_SCHEME + '://' + window.location.hostname + import.meta.env.VITE_PORT;
      document.addEventListener( "keydown", this.handleKeyDown );
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
        console.log("route change2");
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
        if(this.selectedIndex==0) {
          window.scrollTo(0,0);
        }
        this.$axios
        .get(this.host + '/api/search?query=' + encodeURIComponent(param) + '&limit=' + this.pageSize + '&start='+ ((this.page-1)*this.pageSize))
        .then(response =>
          (this.books = response.data.payload,
          this.count = response.data.count,
          this.lastquery = response.data.query,
          this.position = response.data.position,
          this.$emit('bookSearch', response.data.query),
          this.zeroResult(),
          this.gotoIndex()
          )
        )
      },
      gotoIndex() {
        //if(this.selectedIndex!=0) {
            location.hash = "#" + this.selectedIndex;
        //}
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
        this.$router.push({ name: 'books', params: { search:param} });
      },
      nextByKey (topage) {
        console.log("topage:" + topage + ":this.page:" + this.page);
        if(topage>this.page) {
          this.selectedIndex=0;
        } else {
          this.selectedIndex=this.pageSize-1;
        }
        this.page=topage;
        this.dosearchof(this.lastquery);
      },
      next(topage) {
        this.page=topage;
        this.selectedIndex=0;
        this.dosearchof(this.lastquery);
      },
      scrollToCard() {
        const cardRef = this.$refs[`card-${this.selectedIndex}`]
        if (cardRef && cardRef[0]) {
          //cardRef[0].$el.scrollIntoView({ behavior: 'smooth', block: 'start' })
          const cardPosition = cardRef[0].$el.getBoundingClientRect().top + window.scrollY - this.appBarHeight;
          console.log("pos:" + cardPosition + ":index:" + this.selectedIndex + ":appBarY:" + this.appBarHeight);
          window.scrollTo({top: cardPosition, behaviour: "smooth"});
        }
      },
      handlePageDown() {
        this.selectedIndex=this.selectedIndex+1;
          if(this.selectedIndex>this.pageSize) {
            this.nextByKey(this.page+1);
          } else {
            this.scrollToCard();
          }
      },
      handlePageUp() {
        this.selectedIndex=this.selectedIndex-1;
        if(this.selectedIndex<=0) {
          this.nextByKey(this.page-1);
        } else {
          this.scrollToCard();
        }
      },
      handleKeyDown(event) {
        if (event.key === 'PageDown') {
          event.preventDefault();
          this.handlePageDown();
        } else if (event.key === 'PageUp') {
          event.preventDefault();
          this.handlePageUp();
        }
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
