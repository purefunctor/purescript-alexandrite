module Main where

import Library (class Render)
--                    $@

foreign import render :: forall a. Render a => a -> String
