import copy
import datetime
import hashlib
import unittest

import sign_iphone


class SignIphoneTests(unittest.TestCase):
    def setUp(self):
        certificate = b"test certificate"
        self.identity = hashlib.sha1(certificate).hexdigest().upper()
        self.profile = {
            "Entitlements": {
                "application-identifier": "78Z6KJV687.*",
                "com.apple.developer.team-identifier": "78Z6KJV687",
            },
            "ExpirationDate": datetime.datetime(2099, 1, 1),
            "ApplicationIdentifierPrefix": ["78Z6KJV687"],
            "ProvisionedDevices": ["test-phone"],
            "DeveloperCertificates": [certificate],
        }

    def test_accepts_matching_wildcard_and_explicit_profile(self):
        self.assertEqual(self.match(self.profile), self.identity)
        self.profile["Entitlements"]["application-identifier"] = "78Z6KJV687.com.iamnoah.agent"
        self.assertEqual(self.match(self.profile), self.identity)

    def test_rejects_another_app_or_team(self):
        for key, value in (
            ("application-identifier", "78Z6KJV687.com.example.other"),
            ("com.apple.developer.team-identifier", "OTHERTEAM"),
        ):
            with self.subTest(key=key):
                profile = copy.deepcopy(self.profile)
                profile["Entitlements"][key] = value
                self.assertIsNone(self.match(profile))

    def test_rejects_expired_store_and_unsigned_profiles(self):
        for key, value in (
            ("ExpirationDate", datetime.datetime(2000, 1, 1)),
            ("ProvisionedDevices", []),
            ("DeveloperCertificates", [b"unavailable identity"]),
        ):
            with self.subTest(key=key):
                profile = copy.deepcopy(self.profile)
                profile[key] = value
                self.assertIsNone(self.match(profile))

    def match(self, profile):
        return sign_iphone._matching_identity(profile, "com.iamnoah.agent", {self.identity})


if __name__ == "__main__":
    unittest.main()
