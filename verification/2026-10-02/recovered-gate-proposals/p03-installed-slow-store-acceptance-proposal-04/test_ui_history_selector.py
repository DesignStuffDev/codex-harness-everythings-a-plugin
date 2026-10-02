"""Typed selector checks only; real Launch and durability remain separate."""

import copy
import unittest

from slow_store_relay import selected_user_completion
from synthetic_wire_smoke import canonical_event, canonical_payload, nonmatching_payloads, persisted_payload


class UiHistorySelectorTests(unittest.TestCase):
    def setUp(self):
        self.header = {
            "component": {"kind": "thread_store", "name": "default"},
            "method": "thread_store/call",
            "is_control": False,
        }

    def select(self, body, header=None):
        return selected_user_completion(
            self.header if header is None else header,
            body,
            "synthetic-gate-target",
            "synthetic-thread",
        )

    def test_exact_completed_user_item_selects_stable_ids(self):
        self.assertEqual(
            self.select(canonical_payload()),
            {"turn_id": "synthetic-turn", "item_id": "synthetic-item",
             "canonical_event": "ItemCompleted(UserMessage)"},
        )

    def test_raw_response_wrong_types_and_inexact_prompts_do_not_select(self):
        for label, body in nonmatching_payloads():
            with self.subTest(label=label):
                self.assertIsNone(self.select(body))

    def test_wrong_append_thread_and_missing_ids_do_not_select(self):
        body = canonical_payload()
        body["append_items"]["thread_id"] = "other-thread"
        self.assertIsNone(self.select(body))
        for key in ("turn_id", "id"):
            body = canonical_payload()
            event = canonical_event(body)
            (event if key == "turn_id" else event["item"]["UserMessage"])[key] = ""
            with self.subTest(key=key):
                self.assertIsNone(self.select(body))

    def test_control_wrong_method_or_wrong_component_do_not_select(self):
        for key, value in (
            ("is_control", True),
            ("method", "thread_store/open"),
            ("component", {"kind": "attachment_store", "name": "default"}),
        ):
            header = dict(self.header, **{key: value})
            with self.subTest(key=key):
                self.assertIsNone(self.select(canonical_payload(), header))

    def test_multi_part_and_non_text_inputs_do_not_select(self):
        body = canonical_payload()
        content = canonical_event(body)["item"]["UserMessage"]["content"]
        content.append(copy.deepcopy(content[0]))
        self.assertIsNone(self.select(body))
        content.pop()
        content[0]["Image"] = content[0].pop("Text")
        self.assertIsNone(self.select(body))

    def test_persisted_rollout_shape_is_explicitly_rejected(self):
        self.assertIsNone(self.select(persisted_payload()))

    def test_multiple_enum_tags_or_mixed_persisted_wire_shape_do_not_select(self):
        for depth in range(5):
            body = canonical_payload()
            wrappers = [body, body["append_items"]["items"][0],
                        body["append_items"]["items"][0]["EventMsg"],
                        canonical_event(body)["item"],
                        canonical_event(body)["item"]["UserMessage"]["content"][0]]
            wrappers[depth]["type"] = "unexpected-persisted-tag"
            with self.subTest(depth=depth):
                self.assertIsNone(self.select(body))

    def test_missing_text_elements_does_not_select(self):
        body = canonical_payload()
        del canonical_event(body)["item"]["UserMessage"]["content"][0]["Text"]["text_elements"]
        self.assertIsNone(self.select(body))


if __name__ == "__main__":
    unittest.main()
