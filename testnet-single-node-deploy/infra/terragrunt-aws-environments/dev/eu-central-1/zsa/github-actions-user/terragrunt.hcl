locals {
  # Automatically load environment-level variables
  environment_vars = read_terragrunt_config(find_in_parent_folders("env.hcl"))

  region_vars  = read_terragrunt_config(find_in_parent_folders("region.hcl"))
  account_vars = read_terragrunt_config(find_in_parent_folders("account.hcl"))
  # Extract out common variables for reuse
  env = local.environment_vars.locals.environment
}

# Use the terraform-aws-modules IAM user module for creating programmatic access.
# https://github.com/terraform-aws-modules/terraform-aws-iam/tree/master/modules/iam-user
terraform {
  source = "git::https://github.com/terraform-aws-modules/terraform-aws-iam.git//modules/iam-user?ref=v6.6.0"
}

# Include all settings from the root terragrunt.hcl file
include {
  path = find_in_parent_folders()
}

inputs = {
  name = "${local.env}-zebra-github-actions-user"

  create_login_profile = false
  create_access_key    = true
  access_key_status    = "Active"

  create_inline_policy = true
  inline_policy_permissions = {
    AllowECRActions = {
      actions = [
        "ecr:GetAuthorizationToken",
        "ecr:BatchCheckLayerAvailability",
        "ecr:PutImage",
        "ecr:InitiateLayerUpload",
        "ecr:UploadLayerPart",
        "ecr:BatchGetImage",
        "ecr:GetDownloadUrlForLayer",
        "ecr:CompleteLayerUpload"
      ]
      resources = ["*"]
    }
    AllowECRPublicActions = {
      actions = [
        "ecr-public:GetAuthorizationToken",
        "sts:GetServiceBearerToken",
        "ecr-public:PutImage",
        "ecr-public:BatchCheckLayerAvailability",
        "ecr-public:InitiateLayerUpload",
        "ecr-public:UploadLayerPart",
        "ecr-public:CompleteLayerUpload"
      ]
      resources = ["*"]
    }
    AllowECSActions = {
      actions = [
        "ecs:UpdateService",
        "ecs:DescribeServices",
        "ecs:ListServices",
        "ecs:ListTasks",
        "ecs:DescribeTasks",
        "ecs:DescribeTaskDefinition",
        "ecs:RegisterTaskDefinition",
        "ecs:RunTask",
        "ecs:StopTask",
        "ecs:StartTask"
      ]
      resources = ["*"]
    }
    AllowLambdaActions = {
      actions = ["lambda:*"]
      resources = [
        "arn:aws:lambda:${local.region_vars.locals.aws_region}:${local.account_vars.locals.aws_account_id}:function:watch-zebra-logs"
      ]
    }
    AllowPassRole = {
      actions = ["iam:PassRole"]
      resources = [
        "arn:aws:iam::${local.account_vars.locals.aws_account_id}:role/${local.env}-zebra-ecs_execution_role",
        "arn:aws:iam::${local.account_vars.locals.aws_account_id}:role/${local.env}-zebra-swaps-ecs_execution_role"
      ]
    }
  }

  tags = {
    Environment = local.env
    Terraform   = "true"
  }
}
