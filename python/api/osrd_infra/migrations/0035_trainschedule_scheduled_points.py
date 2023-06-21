# Generated by Django 4.1.5 on 2023-06-06 10:04

import osrd_schemas.train_schedule
from django.db import migrations, models

import osrd_infra.utils


class Migration(migrations.Migration):
    dependencies = [
        ("osrd_infra", "0034_rjs_rollingstock_3_2"),
    ]

    operations = [
        migrations.AddField(
            model_name="trainschedule",
            name="scheduled_points",
            field=models.JSONField(
                default=list,
                validators=[osrd_infra.utils.PydanticValidator(osrd_schemas.train_schedule.ScheduledPoints)],
            ),
        ),
    ]