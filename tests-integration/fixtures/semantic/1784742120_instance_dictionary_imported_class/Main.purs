module Main where

import Classes (class Child, class Parent)

data Token = Token

instance childToken :: Parent Token => Child Token where
  child = Token
