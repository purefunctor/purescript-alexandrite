module Main where

import Lib (class Fun)
--                   ^

import Lib (class )
--                ^

import ReExport (class Fun)
--                        ^

import Lib
  ( class Fun
--           ^
  )

import Lib hiding (class Fun)
--                          ^

import Lib (class Fun) as Library
--                   ^

import Lib (class Functor)
--                   ^

import Li
--      ^
