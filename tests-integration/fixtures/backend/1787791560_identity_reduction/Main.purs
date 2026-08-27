module Main where

import Control.Category as Category
import Lookalike as Lookalike

functionIdentity :: Int
functionIdentity = Category.identity 42

lookalikeIdentity :: Int
lookalikeIdentity = Lookalike.identity 42
