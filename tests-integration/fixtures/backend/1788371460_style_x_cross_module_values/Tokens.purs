module Tokens (rowMarker, variables) where

import Alexandrite.StyleX as StyleX

variables = StyleX.defineVars { accent: "blue" }

rowMarker :: StyleX.Marker
rowMarker = StyleX.defineMarker
