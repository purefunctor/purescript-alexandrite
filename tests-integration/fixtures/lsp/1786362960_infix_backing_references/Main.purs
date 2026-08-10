module Main where

import Library as Library

combine left right = left

infixl 6 combine as <+>
--       @$%&?/     $@

type Wrapped = Int

infixl 6 type Wrapped as :+:
--            @$%&?/     $@

infixl 6 Library.combine as <++>
--               @$%&?/

infixl 6 type Library.Wrapped as :++:
--                    @$%&?/

infixl 6 missing as <?>
--       @$%&?/

infixl 6 type Missing as :?:
--            @$%&?/
