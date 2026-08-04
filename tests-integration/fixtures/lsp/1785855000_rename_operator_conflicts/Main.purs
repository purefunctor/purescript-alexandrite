module Main where

renamed left right = left

infixl 5 renamed as <~>

original left right = left

infixl 5 original as <?>
--                   /

type Renamed = Int

infixl 5 type Renamed as <~>

type Original = Int

infixl 5 type Original as :+:
--                        /
