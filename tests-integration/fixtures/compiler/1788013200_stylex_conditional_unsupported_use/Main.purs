module Main where

import Alexandrite.StyleX (Style, conditional)

partialConditional :: Style -> Style
partialConditional = conditional true
