 <template>
  <v-app id="shelfcontrol">
    <v-app-bar color="amber darken-1" clipped-left app>
      <v-app-bar-nav-icon @click="drawer = !drawer"></v-app-bar-nav-icon>
      <span class="title ml-3 mr-5">Shelf&nbsp;<span class="font-weight-light">Control</span></span>
      <v-text-field
        id="searchField"
      v-model="searchtext"
        solo-inverted
        flat
        hide-details
        clearable
        label="Search"
        @change="dosearch()"
        @keydown.enter="$event.target.blur()"
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
            <v-flex xs6 pa-md-4>
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
                <router-link 
                  :to="item.route"
                  tag="span"
                  style="cursor: pointer"
                >{{ item.text }}
                </router-link>
              </v-list-item-title>
            </v-list-item-content>
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
      searchtext:null,
      drawer:null,
      items: [
        { icon: 'book', text: 'All Books', route: '/books' },
        { divider: true },
        { heading: 'Browse' },
        { icon: 'face', text: 'Authors', route: '/categories/authors' },
        { icon: 'sell', text: 'Tags', route: '/categories/tags' },
        { icon: 'business', text: 'Publishers', route: '/categories/publishers'},
        { icon: 'today', text: 'Year', route: 'categories/years' },
        { divider: true },
        { icon: 'settings', text: 'Settings', route: '/settings' },
        { icon: 'help', text: 'Help', route: '/help' },
      ],
      router:null
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
