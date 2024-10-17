 <template>
  <v-app id="shelfcontrol">
    <v-app-bar color="amber" clipped-left app>
      <v-app-bar-nav-icon @click="drawer = !drawer"></v-app-bar-nav-icon>
      <span class="text-h5 ml-3 mr-5">Shelf&nbsp;<span class="font-weight-light">Control</span></span>
      <v-text-field
        id="searchField"
      v-model="searchtext"
        hide-details
        clearable
        density="comfortable"
        variant="solo" 
        bg-color="grey-darken-3"
        @change="dosearch()"
        @keydown.enter="$event.target.blur()"
        prepend-inner-icon="mdi-magnify"
      ></v-text-field>
      <v-spacer></v-spacer>
    </v-app-bar>
    <v-navigation-drawer
      v-model="drawer"
      color="grey-lighten-2"
      clipped
      app
    >
      <v-spacer class="v-spacer">&nbsp;</v-spacer>
      <v-spacer class="v-spacer">&nbsp;</v-spacer>
      <v-spacer class="v-spacer">&nbsp;</v-spacer>
      <v-list
        dense
      >
        <template v-for="(item, i) in items">
              <v-list-item-title v-if="item.heading">
                &nbsp;{{ item.heading }}
              </v-list-item-title>
          <v-divider
            v-else-if="item.divider"
            class="my-3"
          ></v-divider>
          <v-list-item
            v-else
          >
            <template v-slot:prepend>
              <v-icon>{{ item.icon }}</v-icon>
            </template>

              <v-list-item-title>
                <router-link 
                  :to="item.route"
                  tag="span"
                  style="text-decoration: none; color: inherit; cursor: pointer" 
                ><span class="text-grey-darken-5">{{ item.text }}</span>
                </router-link>
              </v-list-item-title>
          </v-list-item>
        </template>
      </v-list>
    </v-navigation-drawer>
    <v-main>
      <router-view @bookSearch="setSearchField" @categoriesInit="clearSearchField"></router-view>
    </v-main>
  </v-app>
</template>

<script>
  //import router from '@/router'

  export default {
    data: () => ({
      searchtext:"*",
      drawer:null,
      items: [
        { icon: 'mdi-book', text: 'All Books', route: '/books/*' },
        { divider: true },
        { icon: 'mdi-face-man', text: 'Authors', route: '/categories/authors' },
        { icon: 'mdi-tag', text: 'Tags', route: '/categories/tags' },
        { icon: 'mdi-domain', text: 'Publishers', route: '/categories/publishers'},
        { icon: 'mdi-calendar-blank', text: 'Year', route: '/categories/years' },
        { divider: true },
        { icon: 'mdi-cog', text: 'Settings', route: '/settings' },
        { icon: 'mdi-help', text: 'Help', route: '/help' },
      ]
    }),
     mounted () {
      var loadParams = this.$route.params.search;
      if(loadParams==undefined || loadParams.trim()=="") {
          loadParams='*';
      }
       this.$router.push({ name: 'books', params: { search:loadParams} });
     },
    methods: {
      dosearch() {
        if(this.searchtext=="") {
          this.searchtext="*";
        }
        this.$router.push({ name: 'books', params: { search:this.searchtext} });
      },
      setSearchField(term) {
        this.searchtext=term;
      },
      clearSearchField() {
        this.searchtext="";
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
