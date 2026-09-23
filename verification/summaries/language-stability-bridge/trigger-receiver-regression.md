# Trigger receiver closure regression

`trigger_receiver_node_parameter_closes_with_nested_feature_value` isolates the
remaining Actions `accept apayload: Anything via receiver` shape. It runs over a
genuinely closed synthetic KerML dependency with a local Systems anchor population;
no standard-publication receipt is fabricated.

The fixture preserves the nested AcceptAction → StateUsage → TransitionUsage →
AcceptActionUsage ownership, inherited inout payload / input receiver, the
transition's two inputs, and the unnamed receiver NodeParameter. The pinned
NodeParameter production has no explicit direction and owns a FeatureValue whose
FeatureReferenceExpression refers to the inherited receiver. The expression has
its own canonical return parameter. Production code is unchanged.

On `2a3d9cb`'s positive-owner witness fix, the focused command
`cargo test -p agq-sysml-semantics
trigger_receiver_node_parameter_closes_with_nested_feature_value -- --nocapture`
fails with exit 101 after 18.92 seconds. The fixed point is Incomplete with eight
incomplete producer pairs: EffectiveTyping on the trigger/parameters,
`KQ_VALUE_CONTEXT` on the receiver binding, and `KQ_VARIABLE_FEATURING` on its
NodeParameter. There are no arity, invalid, or missing-transition-input findings.

With `AGQ_PRODUCER_CAUSAL_TRACE=1`, FeatureValue(60015) reads the generated
contextual feature's owned populations; MayTimeVary(60015) waits on its own
VariableFeaturing. The ignored trace is
`verification/generated/language-stability-bridge/trigger-receiver-micro.log`.
The Complete assertion remains red for the closure owner to fix; this is not a
publication success or an accepted-language claim.
