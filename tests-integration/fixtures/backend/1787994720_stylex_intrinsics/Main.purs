module Main where

import Alexandrite.StyleX as StyleX
import Data.Function as Function
import Data.Ring as Ring

animation :: StyleX.Keyframes
animation = StyleX.keyframes
  { from: { opacity: 0.0 }
  , to: { opacity: 1.0 }
  }

styles = StyleX.create
  { button:
      { color: "red"
      , padding: 8
      , marginInline: Ring.negate 20
      , opacity: Ring.negate 0.5
      , animationName: animation
      , ":hover": { color: "blue" }
      }
  , label: { fontWeight: 600 }
  }

secondary = StyleX.create
  { root: { backgroundColor: "navy" }
  }

buttonProps :: StyleX.Props
buttonProps = StyleX.props styles.button

styleProps = StyleX.recordProps styles

appliedStyleProps = Function.apply StyleX.recordProps styles

flippedStyleProps = Function.applyFlipped styles StyleX.recordProps

buttonPropsArray :: Boolean -> StyleX.Props
buttonPropsArray highlighted = StyleX.props
  [ styles.button
  , StyleX.conditional highlighted secondary.root
  ]

selectedProps :: Boolean -> Boolean -> StyleX.Props
selectedProps included alternate = StyleX.props
  ( StyleX.conditional included
      if alternate then styles.button
      else styles.label
  )

buttonClassName :: String
buttonClassName = buttonProps.className

labelClassName :: String
labelClassName = styleProps.label.className
