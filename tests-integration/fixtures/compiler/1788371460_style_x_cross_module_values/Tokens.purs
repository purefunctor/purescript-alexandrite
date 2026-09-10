module Tokens (rowMarker, variables) where

import Iris.StyleX as StyleX

variables = StyleX.defineVars { accent: "blue" }

rowMarker :: StyleX.Marker
rowMarker = StyleX.defineMarker
