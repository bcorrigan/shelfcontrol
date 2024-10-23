//this lets us move 
import { ref } from 'vue'

const appBarHeight = ref(0)

export function useAppBarHeight() {
  function setAppBarHeight(height) {
    appBarHeight.value = height
  }

  return {
    appBarHeight,
    setAppBarHeight
  }
}
