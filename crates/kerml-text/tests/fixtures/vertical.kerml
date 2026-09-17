// Root/Core vertical slice; Base and Scalar are Features and therefore Types.
namespace Demo {
    feature Scalar;
    feature Base {
        feature count : $::Demo::Scalar;
    }
    abstract type Derived specializes Base {
        feature count typed by $::Demo::Scalar redefines $::Demo::Base::count;
        feature extra : $::Demo::Scalar subsets $::Demo::Base::count;
    }
}
