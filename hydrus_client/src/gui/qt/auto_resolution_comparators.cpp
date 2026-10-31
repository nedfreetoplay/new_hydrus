#include "auto_resolution.hpp"

// WS-15.1.3a: moved verbatim from bridge.cpp (move-only split).

#include "bridge.hpp"
#include "support/support.hpp"

#include <QApplication>
#include <QCheckBox>
#include <QClipboard>
#include <QComboBox>
#include <QDateTime>
#include <QDialog>
#include <QDialogButtonBox>
#include <QDoubleSpinBox>
#include <QFormLayout>
#include <QHBoxLayout>
#include <QHeaderView>
#include <QInputDialog>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonValue>
#include <QLabel>
#include <QLineEdit>
#include <QListWidget>
#include <QMessageBox>
#include <QPlainTextEdit>
#include <QPushButton>
#include <QSpinBox>
#include <QTableWidget>
#include <QTabWidget>
#include <QTextEdit>
#include <QTimer>
#include <QVBoxLayout>
#include <algorithm>

QComboBox *catalog_combo(const QJsonObject &catalog, const QString &key, QWidget *parent) {
    auto *combo = new QComboBox(parent);

    for (const QJsonValue &entry : catalog.value(key).toArray()) {
        const QJsonObject object = entry.toObject();
        combo->addItem(object.value(QStringLiteral("label")).toString(),
                       object.value(QStringLiteral("value")).toString());
    }

    return combo;
}

void select_combo(QComboBox *combo, const QString &value) {
    const int index = combo->findData(value);

    if (index >= 0) {
        combo->setCurrentIndex(index);
    }
}

QString comparator_pretty(const QJsonObject &comparator) {
    const QString kind = comparator.value(QStringLiteral("kind")).toString();

    if (kind == QLatin1String("group")) {
        const QString mode = comparator.value(QStringLiteral("mode")).toString();
        const int count = comparator.value(QStringLiteral("comparators")).toArray().size();

        return QStringLiteral("%1 group (%2 condition(s))")
            .arg(mode == QLatin1String("any") ? QStringLiteral("OR") : QStringLiteral("AND"))
            .arg(count);
    }

    if (kind == QLatin1String("filetype")) {
        return comparator.value(QStringLiteral("same")).toBool(true)
                   ? QStringLiteral("A and B have the same filetype")
                   : QStringLiteral("A and B have different filetypes");
    }

    const QString property = comparator.value(QStringLiteral("property")).toString();
    const QString op = comparator.value(QStringLiteral("operator")).toString(QStringLiteral(">"));

    if (kind == QLatin1String("relative")) {
        const double multiplier = comparator.value(QStringLiteral("multiplier")).toDouble(1.0);
        const double delta = comparator.value(QStringLiteral("delta")).toDouble(0.0);
        QString right = QStringLiteral("B");

        if (qAbs(multiplier - 1.0) > 0.000001) {
            right = QStringLiteral("%1x %2").arg(multiplier).arg(right);
        }

        if (delta > 0.0) {
            right += QStringLiteral(" +%1").arg(delta);
        } else if (delta < 0.0) {
            right += QStringLiteral(" -%1").arg(-delta);
        }

        return QStringLiteral("A %1 %2 %3").arg(property, op, right);
    }

    const QString target = comparator.value(QStringLiteral("looking_at")).toString(QStringLiteral("a"));
    const QString value = property == QLatin1String("mime")
                              ? comparator.value(QStringLiteral("text")).toString()
                              : QString::number(comparator.value(QStringLiteral("value")).toDouble());

    return QStringLiteral("%1 %2 %3 %4")
        .arg(target == QLatin1String("b") ? QStringLiteral("B")
             : target == QLatin1String("either") ? QStringLiteral("either")
                                                 : QStringLiteral("A"),
             property,
             op,
             value);
}

QJsonObject edit_group_dialog(QWidget *parent, const QJsonObject &initial, const QJsonObject &catalog);

QJsonObject edit_condition_dialog(QWidget *parent, const QJsonObject &initial, const QJsonObject &catalog) {
    QDialog dialog(parent);
    dialog.setWindowTitle(QStringLiteral("edit comparison"));
    dialog.resize(460, 300);

    auto *layout = new QVBoxLayout(&dialog);

    auto *kind = new QComboBox(&dialog);
    kind->addItem(QStringLiteral("one file condition"), QStringLiteral("one_file"));
    kind->addItem(QStringLiteral("compare A and B"), QStringLiteral("relative"));
    kind->addItem(QStringLiteral("filetype check"), QStringLiteral("filetype"));
    select_combo(kind, initial.value(QStringLiteral("kind")).toString(QStringLiteral("one_file")));
    layout->addWidget(kind);

    auto *one_file = new QWidget(&dialog);
    auto *one_form = new QFormLayout(one_file);
    auto *target = catalog_combo(catalog, QStringLiteral("looking_at"), one_file);
    auto *property = catalog_combo(catalog, QStringLiteral("properties"), one_file);
    auto *operator_combo = catalog_combo(catalog, QStringLiteral("operators"), one_file);
    auto *value = new QLineEdit(one_file);
    auto *text = new QLineEdit(one_file);
    text->setPlaceholderText(QStringLiteral("image, video, png, image/jpeg ..."));
    one_form->addRow(QStringLiteral("test"), target);
    one_form->addRow(QStringLiteral("property"), property);
    one_form->addRow(QStringLiteral("operator"), operator_combo);
    one_form->addRow(QStringLiteral("value"), value);
    one_form->addRow(QStringLiteral("mime / filetype"), text);
    layout->addWidget(one_file);

    auto *relative = new QWidget(&dialog);
    auto *relative_form = new QFormLayout(relative);
    auto *relative_property = catalog_combo(catalog, QStringLiteral("properties"), relative);
    auto *relative_operator = catalog_combo(catalog, QStringLiteral("operators"), relative);
    auto *multiplier = new QDoubleSpinBox(relative);
    multiplier->setRange(0.0, 1000.0);
    multiplier->setDecimals(3);
    multiplier->setValue(1.0);
    auto *delta = new QDoubleSpinBox(relative);
    delta->setRange(-1000000000.0, 1000000000.0);
    delta->setDecimals(2);
    relative_form->addRow(QStringLiteral("property"), relative_property);
    relative_form->addRow(QStringLiteral("operator"), relative_operator);
    relative_form->addRow(QStringLiteral("B multiplier"), multiplier);
    relative_form->addRow(QStringLiteral("B delta"), delta);
    layout->addWidget(relative);

    auto *filetype = new QWidget(&dialog);
    auto *filetype_form = new QFormLayout(filetype);
    auto *same = new QCheckBox(QStringLiteral("A and B have the same filetype"), filetype);
    same->setChecked(initial.value(QStringLiteral("same")).toBool(true));
    filetype_form->addRow(same);
    layout->addWidget(filetype);

    auto update_kind = [kind, one_file, relative, filetype]() {
        const QString current = kind->currentData().toString();
        one_file->setVisible(current == QLatin1String("one_file"));
        relative->setVisible(current == QLatin1String("relative"));
        filetype->setVisible(current == QLatin1String("filetype"));
    };
    QObject::connect(kind, &QComboBox::currentIndexChanged, &dialog, [update_kind](int) { update_kind(); });

    select_combo(target, initial.value(QStringLiteral("looking_at")).toString(QStringLiteral("a")));
    select_combo(property, initial.value(QStringLiteral("property")).toString(QStringLiteral("size")));
    select_combo(relative_property, initial.value(QStringLiteral("property")).toString(QStringLiteral("size")));
    select_combo(operator_combo, initial.value(QStringLiteral("operator")).toString(QStringLiteral(">")));
    select_combo(relative_operator, initial.value(QStringLiteral("operator")).toString(QStringLiteral(">")));
    value->setText(QString::number(initial.value(QStringLiteral("value")).toDouble()));
    text->setText(initial.value(QStringLiteral("text")).toString());
    multiplier->setValue(initial.value(QStringLiteral("multiplier")).toDouble(1.0));
    delta->setValue(initial.value(QStringLiteral("delta")).toDouble(0.0));

    update_kind();

    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, &dialog);
    QObject::connect(buttons, &QDialogButtonBox::accepted, &dialog, &QDialog::accept);
    QObject::connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);
    layout->addWidget(buttons);

    if (dialog.exec() != QDialog::Accepted) {
        return {};
    }

    const QString current = kind->currentData().toString();
    QJsonObject result;
    result.insert(QStringLiteral("kind"), current);

    if (current == QLatin1String("one_file")) {
        result.insert(QStringLiteral("looking_at"), target->currentData().toString());
        result.insert(QStringLiteral("property"), property->currentData().toString());
        result.insert(QStringLiteral("operator"), operator_combo->currentData().toString());
        result.insert(QStringLiteral("value"), value->text().trimmed().toDouble());
        result.insert(QStringLiteral("text"), text->text().trimmed());
    } else if (current == QLatin1String("relative")) {
        result.insert(QStringLiteral("property"), relative_property->currentData().toString());
        result.insert(QStringLiteral("operator"), relative_operator->currentData().toString());
        result.insert(QStringLiteral("multiplier"), multiplier->value());
        result.insert(QStringLiteral("delta"), delta->value());
    } else {
        result.insert(QStringLiteral("same"), same->isChecked());
    }

    return result;
}

QJsonObject edit_group_dialog(QWidget *parent, const QJsonObject &initial, const QJsonObject &catalog) {
    QDialog dialog(parent);
    dialog.setWindowTitle(QStringLiteral("edit comparison group"));
    dialog.resize(560, 420);

    auto *layout = new QVBoxLayout(&dialog);
    auto *form = new QFormLayout();
    auto *mode = catalog_combo(catalog, QStringLiteral("group_modes"), &dialog);
    select_combo(mode, initial.value(QStringLiteral("mode")).toString(QStringLiteral("all")));
    form->addRow(QStringLiteral("group mode"), mode);
    layout->addLayout(form);

    auto *list = new QListWidget(&dialog);
    layout->addWidget(list, 1);

    QJsonArray comparators = initial.value(QStringLiteral("comparators")).toArray();

    auto refresh = [list, &comparators]() {
        list->clear();

        for (const QJsonValue &value : comparators) {
            auto *item = new QListWidgetItem(comparator_pretty(value.toObject()), list);
            item->setData(Qt::UserRole, QString::fromUtf8(
                                           QJsonDocument(value.toObject()).toJson(QJsonDocument::Compact)));
        }
    };

    auto *buttons_row = new QHBoxLayout();
    auto *add = new QPushButton(QStringLiteral("add condition"), &dialog);
    auto *edit = new QPushButton(QStringLiteral("edit"), &dialog);
    auto *remove = new QPushButton(QStringLiteral("remove"), &dialog);
    buttons_row->addWidget(add);
    buttons_row->addWidget(edit);
    buttons_row->addWidget(remove);
    buttons_row->addStretch(1);
    layout->addLayout(buttons_row);

    QObject::connect(add, &QPushButton::clicked, &dialog, [&]() {
        const QJsonObject condition = edit_condition_dialog(&dialog, QJsonObject(), catalog);

        if (!condition.isEmpty()) {
            comparators.append(condition);
            refresh();
        }
    });

    QObject::connect(edit, &QPushButton::clicked, &dialog, [&]() {
        const int row = list->currentRow();

        if (row < 0) {
            return;
        }

        const QJsonObject current = comparators.at(row).toObject();
        const QJsonObject changed = current.value(QStringLiteral("kind")).toString() == QLatin1String("group")
                                        ? edit_group_dialog(&dialog, current, catalog)
                                        : edit_condition_dialog(&dialog, current, catalog);

        if (!changed.isEmpty()) {
            comparators.replace(row, changed);
            refresh();
        }
    });

    QObject::connect(remove, &QPushButton::clicked, &dialog, [&]() {
        const int row = list->currentRow();

        if (row >= 0) {
            comparators.removeAt(row);
            refresh();
        }
    });

    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, &dialog);
    QObject::connect(buttons, &QDialogButtonBox::accepted, &dialog, &QDialog::accept);
    QObject::connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);
    layout->addWidget(buttons);

    refresh();

    if (dialog.exec() != QDialog::Accepted || comparators.isEmpty()) {
        return {};
    }

    QJsonObject result;
    result.insert(QStringLiteral("kind"), QStringLiteral("group"));
    result.insert(QStringLiteral("mode"), mode->currentData().toString());
    result.insert(QStringLiteral("comparators"), comparators);

    return result;
}
