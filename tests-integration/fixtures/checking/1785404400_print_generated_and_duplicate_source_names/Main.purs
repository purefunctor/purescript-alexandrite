module Main where

generatedIdentity value = value

generatedConst first _ = first

generatedApply function value = function value

sourceIdentity :: forall value. value -> value
sourceIdentity value = value

sourceConst :: forall first second. first -> second -> first
sourceConst first _ = first

sourceNumeric :: forall t0 t1. t0 -> t1 -> t0
sourceNumeric first _ = first

sourceShadowed :: forall value. value -> (forall value. value -> value) -> value
sourceShadowed value _ = value

data Proxy :: forall kind. kind -> Type
data Proxy value = Proxy

type GeneratedKind0 = Proxy

type GeneratedKind1 value = Proxy

type GeneratedKind2 first second = Proxy
