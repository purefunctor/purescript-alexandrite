module Main where

import Library (Box(..), box, libraryValue)

unbox :: Box Int -> Int
unbox (Box value) = value

fromLibrary :: Int
fromLibrary = unbox box

directReference :: Int
directReference = libraryValue
