# Generated by Django 4.1.5 on 2023-08-01 12:43

from django.db import migrations, models


class Migration(migrations.Migration):

    dependencies = [
        ("osrd_infra", "0042_remove_backside_pantograph_track_ranges"),
    ]

    operations = [
        migrations.RenameModel(
            old_name="DeadSectionLayer",
            new_name="NeutralSectionLayer",
        ),
        migrations.RenameModel(
            old_name="DeadSectionModel",
            new_name="NeutralSectionModel",
        ),
        migrations.AlterModelOptions(
            name="neutralsectionlayer",
            options={"verbose_name_plural": "generated neutral section layer"},
        ),
        migrations.AlterModelOptions(
            name="neutralsectionmodel",
            options={"verbose_name_plural": "neutral sections"},
        ),
        migrations.RunSQL(
            sql=[("ALTER TABLE osrd_infra_deadsectionmodel_id_seq RENAME TO osrd_infra_neutralsectionmodel_id_seq;")],
            reverse_sql=[
                ("ALTER TABLE osrd_infra_neutralsectionmodel_id_seq RENAME TO osrd_infra_deadsectionmodel_id_seq;")
            ],
        ),
    ]