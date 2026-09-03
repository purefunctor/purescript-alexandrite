module Main where

import Alexandrite.StyleX as StyleX
import Alexandrite.StyleX.Types as Types
import Alexandrite.StyleX.When as When

constants = StyleX.defineConsts
  { compact: "@media (max-width: 40rem)"
  , columns: 12
  }

variables = StyleX.defineVars
  { accent: Types.color "royalblue"
  , angle: Types.angle "45deg"
  , image: Types.image "linear-gradient(red, blue)"
  , integer: Types.integer 1
  , length: Types.length "8px"
  , lengthPercentage: Types.lengthPercentage "10%"
  , number: Types.number 0.5
  , percentage: Types.percentage "50%"
  , resolution: Types.resolution "2dppx"
  , time: Types.time "200ms"
  , transformFunction: Types.transformFunction "scale(1)"
  , transformList: Types.transformList "scale(1) rotate(2deg)"
  , url: Types.url "url(image.png)"
  }

theme = StyleX.createTheme variables
  { accent: Types.color "white"
  , length: Types.length "12px"
  }

rowMarker :: StyleX.Marker
rowMarker = StyleX.defineMarker

styles = StyleX.create
  { root:
      { color: StyleX.conditionalValue "blue"
          [ When.ancestor ":hover" "red"
          , When.ancestorMarker ":focus" rowMarker "green"
          , When.descendant ":hover" "purple"
          , When.descendantMarker ":focus" rowMarker "pink"
          , When.siblingBefore ":hover" "orange"
          , When.siblingBeforeMarker ":focus" rowMarker "yellow"
          , When.siblingAfter ":hover" "gray"
          , When.siblingAfterMarker ":focus" rowMarker "black"
          , When.anySibling ":hover" "navy"
          , When.anySiblingMarker ":focus" rowMarker "teal"
          ]
      , position: StyleX.firstThatWorks [ "sticky", "fixed" ]
      }
  }

attributes :: StyleX.Attrs
attributes = StyleX.attrs styles.root

recordAttributes :: { root :: StyleX.Attrs }
recordAttributes = StyleX.recordAttrs styles

markerProps :: StyleX.Props
markerProps = StyleX.props rowMarker

defaultMarkerProps :: StyleX.Props
defaultMarkerProps = StyleX.props StyleX.defaultMarker

transitionClass :: String
transitionClass = StyleX.viewTransitionClass
  { "new": { opacity: 1.0 }
  , old: { opacity: 0.0 }
  }

fallback :: String
fallback = StyleX.positionTry
  { positionArea: "block-start"
  , margin: 8
  }
