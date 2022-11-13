#!/usr/bin/env python

import argparse
from typing import List
import json
from pathlib import Path
import re


def sh_str(s: str) -> str:
    return f"'{s}'"


def sh_array(l: List[str]) -> str:
    return "(" + " ".join(map(sh_str, l)) + ")"


class PkgBuild:
    pkgname: str
    pkgver: str
    pkgrel: str
    pkgdesc: str
    # arch: List[str]
    url: str
    license: List[str]
    # depends: List[str]
    # makedepends: List[str]
    # source: List[str]
    # sha512sums: List[str]

    def parse_metadata(self, filename: Path):
        with open(filename, encoding="utf-8") as f:
            metadata: dict = json.load(f)

        pkg = metadata["packages"][0]

        self.pkgname = pkg["name"]
        self.pkgver = pkg["version"]
        self.pkgdesc = pkg["description"]
        self.url = pkg["repository"]
        self.license = [pkg["license"]]

    def generate(self, template_path: Path, out_dir: Path):
        with open(template_path, encoding="utf-8") as f:
            template = f.read()

        out = template
        out = re.sub("@@NAME@@", sh_str(self.pkgname), out)
        out = re.sub("@@VERSION@@", sh_str(self.pkgver), out)
        out = re.sub("@@RELEASE@@", sh_str(self.pkgrel), out)
        out = re.sub("@@DESCRIPTION@@", sh_str(self.pkgdesc), out)
        out = re.sub("@@LICENSE@@", sh_array(self.license), out)
        out = re.sub("@@URL@@", sh_str(self.url), out)
        out = re.sub("@@REPO_URL@@", self.url, out)

        with open(out_dir.joinpath("PKGBUILD"), "w", encoding="utf-8") as f:
            f.write(out)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--out-dir",
        help="Directory where to write the PKGBUILD",
        type=Path,
        required=True,
    )
    parser.add_argument(
        "--template", help="Path to PKGBUILD.in", type=Path, default=Path("PKGBUILD.in")
    )
    parser.add_argument(
        "--package-release",
        help="Version of the release (the 1 in version 2.3.4-1)",
        type=str,
        required=True,
    )
    parser.add_argument(
        "--metadata",
        help="Path to JSON metadata produced by cargo",
        type=Path,
        default=Path("metadata.json"),
    )
    args = parser.parse_args()

    pkgbuild = PkgBuild()
    pkgbuild.pkgrel = args.package_release

    pkgbuild.parse_metadata(args.metadata)
    pkgbuild.generate(args.template, args.out_dir)


if __name__ == "__main__":
    main()
