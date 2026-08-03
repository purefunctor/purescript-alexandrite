module Main where

import Alpha as Alpha
import Beta as Beta

instance Alpha.Alpha Int where
  execute value = value
-- $ @

instance Beta.Beta Int where
  execute value = value
-- $ @
