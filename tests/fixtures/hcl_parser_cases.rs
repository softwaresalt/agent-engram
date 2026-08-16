//! Compact source/expected-output matrix for HCL parser harnesses (121.002.001-ST).

pub struct HclParserCase {
    pub name: &'static str,
    pub source: &'static str,
    pub symbols: &'static [&'static str],
    pub references: &'static [&'static str],
}

pub const DECLARATIONS: HclParserCase = HclParserCase {
    name: "plain block labels and top-level attributes",
    source: r#"
terraform {
  required_version = ">= 1.6"
}

resource "aws_instance" "web" {
  ami = "ami-123"
}

data "aws_ami" "ubuntu" {
  most_recent = true
}

module "vpc" {
  source = "./vpc"
}

region = "us-west-2"
"#,
    symbols: &[
        "hcl.block.terraform",
        "hcl.block.resource.aws_instance.web",
        "hcl.block.data.aws_ami.ubuntu",
        "hcl.block.module.vpc",
        "hcl.attribute.region",
    ],
    references: &[],
};

pub const TRAVERSALS: HclParserCase = HclParserCase {
    name: "plain traversal dedup and dynamic-form skips",
    source: r#"
resource "aws_instance" "web" {
  region           = var.region
  duplicate_region = var.region
  subnet_id        = module.vpc.id
  image_id         = data.aws_ami.ubuntu.id
  indexed          = local.items[count.index]
  splatted         = aws_instance.web[*].id
  rendered         = "${local.name}"
}
"#,
    symbols: &["hcl.block.resource.aws_instance.web"],
    references: &["var.region", "module.vpc.id", "data.aws_ami.ubuntu.id"],
};

pub const MALFORMED: HclParserCase = HclParserCase {
    name: "malformed traversal fails closed",
    source: r#"
resource "aws_instance" "broken" {
  ami = var.
"#,
    symbols: &[],
    references: &[],
};
