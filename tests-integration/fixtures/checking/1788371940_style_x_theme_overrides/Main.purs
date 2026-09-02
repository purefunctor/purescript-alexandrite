module Main where

import Alexandrite.StyleX as StyleX
import Alexandrite.StyleX.Types as Types

variables = StyleX.defineVars
  { accent: Types.color "blue"
  , spacing: Types.length "8px"
  }

valid = StyleX.createTheme variables
  { accent: Types.color "white" }

wrongType = StyleX.createTheme variables
  { accent: Types.length "12px" }

unknown = StyleX.createTheme variables
  { missing: Types.color "red" }

invalidInteger = StyleX.defineVars { value: Types.integer true }

invalidNumber = StyleX.defineVars { value: Types.number "1" }

invalidColor = StyleX.defineVars { value: Types.color 42 }
