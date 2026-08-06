module Main where

import Lib (class Fu)
--                  ^

import Lib (class  Foldable)
--                ^

import Lib (class
--               ^
  -- Keep this comment.
  Fu)

import Lib (class Fu)
--            ^

import Lib (class
  -- Keep this comment.
--     ^
  Fu)

import Lib (class Foldable, class)
--                               ^

import Lib hiding (class Foldable, class)
--                                      ^

import Lib (class Functor, class Foldable)
--                                      ^
