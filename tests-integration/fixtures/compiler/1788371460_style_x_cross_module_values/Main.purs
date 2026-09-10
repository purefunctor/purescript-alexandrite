module Main where

import Iris.StyleX as StyleX
import Iris.StyleX.When as When
import Tokens (rowMarker, variables)

theme = StyleX.createTheme variables { accent: "white" }

styles = StyleX.create
  { root:
      { color: StyleX.conditionalValue "blue"
          [ When.ancestorMarker ":hover" rowMarker "red" ]
      }
  }
