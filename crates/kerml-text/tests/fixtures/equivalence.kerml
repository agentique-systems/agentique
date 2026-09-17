namespace N {
    feature Base { feature f; }
    type Derived :> Base {
        feature g : $::N::Base :>> $::N::Base::f;
    }
}
