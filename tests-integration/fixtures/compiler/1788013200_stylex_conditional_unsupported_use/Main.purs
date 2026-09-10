module Main where

import Iris.StyleX (Style, conditional)

partialConditional :: Style -> Style
partialConditional = conditional true
