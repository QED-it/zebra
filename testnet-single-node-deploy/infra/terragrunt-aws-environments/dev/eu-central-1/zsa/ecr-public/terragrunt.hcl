locals {
  # Automatically load environment-level variables
  environment_vars = read_terragrunt_config(find_in_parent_folders("env.hcl"))

  region_vars  = read_terragrunt_config(find_in_parent_folders("region.hcl"))
  account_vars = read_terragrunt_config(find_in_parent_folders("account.hcl"))
  # Extract out common variables for reuse
  env = local.environment_vars.locals.environment
}

# Terragrunt will copy the Terraform configurations specified by the source parameter, along with any files in the
# working directory, into a temporary folder, and execute your Terraform commands in that folder.
terraform {
  source = "git::https://github.com/terraform-aws-modules/terraform-aws-ecr.git?ref=v2.4.0"
}

# Include all settings from the root terragrunt.hcl file
include {
  path           = find_in_parent_folders()
  merge_strategy = "deep"
}

generate "provider" {
  path      = "provider.tf"
  if_exists = "overwrite_terragrunt"
  contents  = <<EOF
provider "aws" {
  # ECR Public is global, but its API is only available through us-east-1.
  region = "us-east-1"
  profile = "${local.account_vars.locals.aws_profile}"
  allowed_account_ids = ["${local.account_vars.locals.aws_account_id}"]
}
EOF
}

inputs = {
  repository_type = "public"
  repository_name = "zebra-server"

  create_repository_policy = false
  repository_policy = jsonencode({
    Version = "2008-10-17"
    Statement = [
      {
        Sid       = "AllowPublicPull"
        Effect    = "Allow"
        Principal = "*"
        Action = [
          "ecr-public:GetRepositoryCatalogData",
          "ecr-public:BatchCheckLayerAvailability",
          "ecr-public:GetDownloadUrlForLayer",
          "ecr-public:BatchGetImage"
        ]
      }
    ]
  })

  public_repository_catalog_data = {
    about_text        = "QEDIT Zebra Server"
    architectures     = ["ARM"]
    description       = "QEDIT Zebra Server is a Zcash node for the ZSA testnet"
    operating_systems = ["Linux"]
    usage_text        = "Run the Docker image with the Zebra configuration for the ZSA testnet"
  }

  tags = {
    env       = local.env
    Terraform = "true"
  }
}
