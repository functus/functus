import importlib.util
import unittest
from pathlib import Path


HOOK_PATH = Path(__file__).parents[1] / "tdd_guard.py"
SPEC = importlib.util.spec_from_file_location("tdd_guard", HOOK_PATH)
tdd_guard = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(tdd_guard)


class ChangedHunkLineTests(unittest.TestCase):
    def test_context_test_marker_is_not_a_change(self) -> None:
        self.assertFalse(tdd_guard.is_changed_hunk_line(" #[test]"))

    def test_changed_test_marker_is_a_change(self) -> None:
        self.assertTrue(tdd_guard.is_changed_hunk_line("+#[test]"))
        self.assertTrue(tdd_guard.is_changed_hunk_line("-#[test]"))

    def test_file_headers_are_not_changes(self) -> None:
        self.assertFalse(tdd_guard.is_changed_hunk_line("+++ b/src/lib.rs"))
        self.assertFalse(tdd_guard.is_changed_hunk_line("--- a/src/lib.rs"))


class TestScopeTests(unittest.TestCase):
    def test_implementation_near_existing_test_is_not_test_change(self) -> None:
        diff = """diff --git a/src/lib.rs b/src/lib.rs
--- a/src/lib.rs
+++ b/src/lib.rs
@@ -1,5 +1,5 @@
 #[test]
 fn existing() { assert!(true); }
-pub fn value() -> i32 { 1 }
+pub fn value() -> i32 { 2 }
"""
        self.assertFalse(tdd_guard.has_test_changes(diff))

    def test_existing_unit_test_body_change_is_detected(self) -> None:
        diff = """diff --git a/src/lib.rs b/src/lib.rs
--- a/src/lib.rs
+++ b/src/lib.rs
@@ -1,5 +1,5 @@
 #[cfg(test)]
 mod tests {
   #[test]
   fn existing() {
-    assert_eq!(value(), 1);
+    assert_eq!(value(), 2);
   }
 }
"""
        self.assertTrue(tdd_guard.has_test_changes(diff))

    def test_braced_proptest_is_detected(self) -> None:
        self.assertTrue(tdd_guard.TEST_MARKERS.search("proptest! {") is not None)

    def test_non_rust_marker_does_not_count(self) -> None:
        diff = """diff --git a/README.md b/README.md
--- a/README.md
+++ b/README.md
@@ -1 +1 @@
-old
+example: #[test]
"""
        self.assertFalse(tdd_guard.has_test_changes(diff))

    def test_brace_in_string_does_not_end_test_scope(self) -> None:
        diff = '''diff --git a/src/lib.rs b/src/lib.rs
--- a/src/lib.rs
+++ b/src/lib.rs
@@ -1,7 +1,7 @@
 #[cfg(test)]
 mod tests {
   #[test]
   fn existing() {
     let template = "}";
-    assert!(false);
+    assert!(true);
   }
 }
'''
        self.assertTrue(tdd_guard.has_test_changes(diff))

    def test_braces_in_char_and_raw_string_do_not_end_test_scope(self) -> None:
        diff = '''diff --git a/src/lib.rs b/src/lib.rs
--- a/src/lib.rs
+++ b/src/lib.rs
@@ -1,8 +1,8 @@
 #[cfg(test)]
 mod tests {
   #[test]
   fn existing() {
     let brace = '}';
     let raw = r#"}"#;
-    assert!(false);
+    assert!(true);
   }
 }
'''
        self.assertTrue(tdd_guard.has_test_changes(diff))

    def test_external_test_module_does_not_mark_next_function(self) -> None:
        diff = '''diff --git a/src/lib.rs b/src/lib.rs
--- a/src/lib.rs
+++ b/src/lib.rs
@@ -1,4 +1,4 @@
 #[cfg(test)]
 mod tests;
-fn production() { old(); }
+fn production() { new(); }
'''
        self.assertFalse(tdd_guard.has_test_changes(diff))


if __name__ == "__main__":
    unittest.main()
