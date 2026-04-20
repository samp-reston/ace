# `ace-proto`

Raw frame wrappers with no protocol knowledge. Provides `UdsFrame<'a>`, `UdsFrameMut<'a>`, `DoipFrame<'a>`, `DoipFrameMut<'a>`, and CAN frame types (`CanFrame`, `CanFrameMut`, `CanFdFrame`, `CanFdFrameMut`).

These types wrap byte slices and provide structural access — length, index, iteration. Protocol semantics are added by extension traits in `ace-uds` and `ace-doip`.
