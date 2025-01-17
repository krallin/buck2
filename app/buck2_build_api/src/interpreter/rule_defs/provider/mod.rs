/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

//! Providers are the data returned from a rule, and are the only way that information from this
//! rule is available to rules that depend on it. Every rule must return at least the `DefaultInfo`
//! provider, but most will also return either `RunInfo` (because they are executable) or some
//! custom provider (because they are incorporated into something that is ultimately executable).
//!
//! Internal providers (those defined and used by buck itself) can be defined easily using the
//! #[internal_provider(creator_func)] macro. This will generate all the code needed for that
//! provider to be used in starlark and to be treated as a provider in the various rust utilities
//! we have for providers.
//!
//! For an internal provider like:
//! ```skip
//! #[internal_provider(create_my_prov)]
//! #[derive(Clone, Debug, Trace, Coerce)]
//! #[repr(transparent)]
//! pub struct MyProviderGen<V> {
//!    field1: V,
//!    field2: V,
//! }
//!
//! #[starlark_module]
//! fn create_my_prov(globals: &mut GlobalsBuilder) {
//!    fn NameDoesntMatter(
//!        // It's not enforced that the args here match the fields, but it's generally the user expectation that they do.
//!        field1: Value<'v>,
//!        field2: Value<'v>,
//!    ) -> MyProvider<'v> {
//!       // Can do some arg validation or computation here, just need to construct the provider.
//!       Ok(MyProvider {
//!            field1,
//!            field2
//!        })
//!    }
//! }
//! ```
//!
//! This will generate a "ProviderCallable" starlark type named (in starlark) `MyProvider` that acts like
//! the instance returned by a `provider()` call in starlark (so can be used to construct instances of the
//! provider or used in places like `attrs.dep(required_providers=[MyProvider]))`.
//!
//! For provider instances, in starlark all of their fields will be accessible by the field name.
//!
//! In rust, a StarlarkValue can be converted to the provider like normal with `MyProvider::from_value()`.
//! Often internally we'd have the analysis result (`FrozenProviderCollection`) and want to get the
//! provider out of their so there's a convenience function for that: `MyProvider::from_providers(collect)`.
// TODO(cjhopman): That last one would be more discoverable if we moved it onto the
// `FrozenProviderCollectionValue` itself so you could do `collection.get::<MyProvider>()`.
use std::fmt::Debug;
use std::sync::Arc;

use buck2_core::provider::id::ProviderId;
use starlark::any::ProvidesStaticType;
use starlark::environment::MethodsBuilder;
use starlark::values::Value;
use starlark::values::ValueLike;

use crate::interpreter::rule_defs::provider::builtin::default_info::DefaultInfo;
use crate::interpreter::rule_defs::provider::builtin::default_info::DefaultInfoCallable;
use crate::interpreter::rule_defs::provider::builtin::default_info::FrozenDefaultInfo;
use crate::interpreter::rule_defs::provider::collection::ProviderCollection;

pub mod builtin;
pub mod callable;
pub mod collection;
pub(crate) mod dependency;
pub mod registration;
pub mod test_provider;
pub(crate) mod user;

pub(crate) trait ProviderLike<'v>: Debug {
    /// The ID. Guaranteed to be set on the `ProviderCallable` before constructing this object
    fn id(&self) -> &Arc<ProviderId>;
    /// Gets the value for a given field.
    fn get_field(&self, name: &str) -> Option<Value<'v>>;
    /// Returns a list of all the keys and values.
    // TODO(cjhopman): I'd rather return an iterator. I couldn't get that to work, though.
    fn items(&self) -> Vec<(&str, Value<'v>)>;
}

unsafe impl<'v> ProvidesStaticType for &'v dyn ProviderLike<'v> {
    type StaticType = &'static dyn ProviderLike<'static>;
}

/// Common methods on user and builtin providers.
#[starlark_module]
pub(crate) fn provider_methods(builder: &mut MethodsBuilder) {
    fn to_json(this: Value) -> anyhow::Result<String> {
        this.to_json()
    }
}

pub(crate) trait ValueAsProviderLike<'v> {
    fn as_provider(&self) -> Option<&'v dyn ProviderLike<'v>>;
}

impl<'v, V: ValueLike<'v>> ValueAsProviderLike<'v> for V {
    fn as_provider(&self) -> Option<&'v dyn ProviderLike<'v>> {
        self.to_value().request_value()
    }
}

#[cfg(test)]
pub mod testing {

    use buck2_interpreter_for_build::attrs::coerce;
    use starlark::environment::GlobalsBuilder;
    use starlark::environment::Module;

    use crate::interpreter::build_defs::register_provider;
    use crate::interpreter::rule_defs::provider::collection::FrozenProviderCollectionValue;
    use crate::interpreter::rule_defs::provider::registration::register_builtin_providers;

    pub trait FrozenProviderCollectionValueExt {
        /// Creates a `FrozenProviderCollectionValue` for testing. The given string should be
        /// Starlark code that returns a list of providers. The built in providers are available.
        fn testing_new(providers: &str) -> Self;
    }

    impl FrozenProviderCollectionValueExt for FrozenProviderCollectionValue {
        fn testing_new(providers: &str) -> Self {
            let env = Module::new();
            let globals = GlobalsBuilder::extended()
                .with(register_builtin_providers)
                .with(register_provider)
                .build();
            let value = coerce::testing::to_value(&env, &globals, providers);
            let res_typed =
                crate::interpreter::rule_defs::provider::ProviderCollection::try_from_value(value)
                    .map_err(|e| anyhow::anyhow!("{:?}", e))
                    .unwrap();

            let res = env.heap().alloc(res_typed);
            env.set("", res);

            let frozen_env = env.freeze().expect("should freeze successfully");
            let res = frozen_env.get("").unwrap();

            FrozenProviderCollectionValue::try_from_value(res)
                .expect("just created this, this shouldn't happen")
        }
    }
}

#[cfg(test)]
mod tests {
    use allocative::Allocative;
    use buck2_build_api_derive::internal_provider;
    use buck2_core::bzl::ImportPath;
    use buck2_interpreter_for_build::interpreter::testing::Tester;
    use indoc::indoc;
    use starlark::any::ProvidesStaticType;
    use starlark::coerce::Coerce;
    use starlark::environment::GlobalsBuilder;
    use starlark::values::Freeze;
    use starlark::values::Trace;
    use starlark::values::Value;

    use crate::interpreter::rule_defs::register_rule_defs;

    #[internal_provider(simple_info_creator)]
    #[derive(Clone, Debug, Trace, Coerce, Freeze, ProvidesStaticType, Allocative)]
    #[repr(C)]
    pub struct SimpleInfoGen<V> {
        value1: V,
        value2: V,
    }

    #[starlark_module]
    fn simple_info_creator(globals: &mut GlobalsBuilder) {
        fn ConstraintSettingInfo<'v>(
            value1: Value<'v>,
            value2: Value<'v>,
        ) -> anyhow::Result<SimpleInfo<'v>> {
            Ok(SimpleInfo { value1, value2 })
        }
    }

    fn provider_tester() -> Tester {
        let mut tester = Tester::new().unwrap();
        tester.set_additional_globals(|builder| {
            simple_info_creator(builder);
            register_rule_defs(builder);
            crate::interpreter::build_defs::register_provider(builder);
        });
        tester
    }

    #[test]
    fn creates_providers() -> anyhow::Result<()> {
        // TODO(nmj): Starlark doesn't let you call 'new_invoker()' on is_mutable types.
        //                 Once that's fixed, make sure we can call 'FooInfo' before the module is
        //                 frozen.
        let mut tester = provider_tester();

        tester.run_starlark_test(indoc!(
            r#"
        FooInfo = provider(fields=["bar", "baz"])
        FooInfo2 = FooInfo
        #frozen_foo_1 = FooInfo(bar="bar_f1", baz="baz_f1")
        #frozen_foo_2 = FooInfo(bar="bar_f2")

        assert_eq("unnamed provider", repr(provider(fields=["f1"])))
        assert_eq("FooInfo(bar, baz)", repr(FooInfo))
        assert_eq("FooInfo(bar, baz)", repr(FooInfo2))

        simple_info_1 = SimpleInfo(value1="value1", value2=3)

        def test():
            assert_eq(FooInfo.type, "FooInfo")
            assert_eq("FooInfo(bar, baz)", repr(FooInfo))
            assert_eq("FooInfo(bar, baz)", repr(FooInfo2))

            #assert_eq("FooInfo(bar=\"bar_f1\", baz=\"baz_f1\")", repr(frozen_foo1))
            #assert_eq("bar_f1", frozen_foo1.bar)
            #assert_eq("baz_f1", frozen_foo1.baz)
            #assert_eq("FooInfo(bar=\"bar_f2\", baz=None)", repr(frozen_foo2))
            #assert_eq("bar_f2", frozen_foo2.bar)
            #assert_eq(None, frozen_foo2.baz)

            foo_1 = FooInfo(bar="bar_1", baz="baz_1")
            foo_2 = FooInfo(bar="bar_2")

            assert_eq("FooInfo(bar, baz)", repr(FooInfo))
            assert_eq("FooInfo(bar=\"bar_1\", baz=\"baz_1\")", repr(foo_1))
            assert_eq("bar_1", foo_1.bar)
            assert_eq("baz_1", foo_1.baz)
            assert_eq("FooInfo(bar=\"bar_2\", baz=None)", repr(foo_2))
            assert_eq("bar_2", foo_2.bar)
            assert_eq(None, foo_2.baz)

            assert_eq("{\"bar\":\"bar_1\",\"baz\":\"baz_1\"}", foo_1.to_json())
            assert_eq("{\"value1\":\"value1\",\"value2\":3}", simple_info_1.to_json())
            assert_eq(json.encode(struct(value1="value1", value2=3)), simple_info_1.to_json())
        "#
        ))?;

        tester.run_starlark_test_expecting_error(
            indoc!(
                r#"
        FooInfo = provider(fields=["bar", "baz"])

        def test():
            foo_1 = FooInfo(bar="bar1")
            foo_1.quz
        "#
            ),
            "Object of type `provider` has no attribute `quz`",
        );

        tester.run_starlark_test_expecting_error(
            indoc!(
                r#"
        list = []
        list.append(provider(fields=["bar", "baz"]))
        "#
            ),
            "must be assigned to a variable",
        );

        // Make sure that frozen UserProvider instances work
        let mut tester = provider_tester();
        tester.add_import(
            &ImportPath::testing_new("root//provider:def1.bzl"),
            indoc!(
                r#"
                FooInfo = provider(fields=["foo"])
                "#
            ),
        )?;
        tester.add_import(
            &ImportPath::testing_new("root//provider:def2.bzl"),
            indoc!(
                r#"
                load("//provider:def1.bzl", "FooInfo")
                foo = FooInfo(foo="foo1")
                "#
            ),
        )?;
        tester.run_starlark_test(indoc!(
            r#"
            load("//provider:def2.bzl", "foo")
            def test():
                assert_eq('FooInfo(foo="foo1")', repr(foo))
            "#
        ))?;

        Ok(())
    }
}
