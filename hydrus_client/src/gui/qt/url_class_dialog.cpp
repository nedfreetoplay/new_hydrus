#include "url_class_dialog.hpp"

// WS-15.1.3c1: url class editor moved from network_tables.cpp (move-only split;
// inline methods outlined, no logic change).

#include "support/support.hpp"

#include <QCheckBox>
#include <QComboBox>
#include <QDialogButtonBox>
#include <QFormLayout>
#include <QHBoxLayout>
#include <QJsonValue>
#include <QLabel>
#include <QLineEdit>
#include <QPlainTextEdit>
#include <QPushButton>
#include <QStringList>
#include <QUrl>
#include <QVBoxLayout>
#include <QWidget>

HydrusUrlClassDialog::HydrusUrlClassDialog(const QJsonObject &entry, const QJsonArray &types, QWidget *parent)
    : QDialog(parent), data(entry) {
    setWindowTitle(QStringLiteral("url class"));
    resize(620, 560);

    auto *layout = new QVBoxLayout(this);
    auto *form = new QFormLayout();

    name = new QLineEdit(this);
    type = new QComboBox(this);
    scheme = new QLineEdit(this);
    netloc = new QLineEdit(this);
    path = new QLineEdit(this);
    parameters = new QPlainTextEdit(this);
    parameters->setMaximumHeight(80);
    parameters->setPlaceholderText(QStringLiteral("one name=value per line"));
    example = new QLineEdit(this);
    match_subdomains = new QCheckBox(QStringLiteral("match subdomains"), this);
    keep_subdomains = new QCheckBox(QStringLiteral("keep the matched subdomain"), this);
    multiple = new QCheckBox(QStringLiteral("can produce multiple files"), this);
    associate = new QCheckBox(QStringLiteral("associate with files"), this);
    associate->setChecked(true);

    for (const QJsonValue &value : types) {
        const QJsonObject item = value.toObject();
        type->addItem(item.value(QStringLiteral("label")).toString(), item.value(QStringLiteral("value")).toInt());
    }

    form->addRow(QStringLiteral("name"), name);
    form->addRow(QStringLiteral("url type"), type);
    form->addRow(QStringLiteral("preferred scheme"), scheme);
    form->addRow(QStringLiteral("domain"), netloc);
    form->addRow(QStringLiteral("path"), path);
    form->addRow(QStringLiteral("parameters"), parameters);
    form->addRow(QStringLiteral("example url"), example);
    form->addRow(match_subdomains);
    form->addRow(keep_subdomains);
    form->addRow(multiple);
    form->addRow(associate);
    layout->addLayout(form);

    auto *test_row = new QWidget(this);
    auto *test_layout = new QHBoxLayout(test_row);
    test_layout->setContentsMargins(0, 0, 0, 0);
    test = new QLineEdit(test_row);
    test->setPlaceholderText(QStringLiteral("paste a url to test matching"));
    auto *test_button = new QPushButton(QStringLiteral("test"), test_row);
    test_layout->addWidget(test, 1);
    test_layout->addWidget(test_button);
    layout->addWidget(test_row);

    result = new QLabel(this);
    result->setWordWrap(true);
    layout->addWidget(result);

    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, this);
    connect(buttons, &QDialogButtonBox::accepted, this, &QDialog::accept);
    connect(buttons, &QDialogButtonBox::rejected, this, &QDialog::reject);
    connect(test_button, &QPushButton::clicked, this, [this]() { run_test(); });
    layout->addWidget(buttons);

    name->setText(data.value(QStringLiteral("name")).toString());
    scheme->setText(data.value(QStringLiteral("preferred_scheme")).toString());
    if (scheme->text().isEmpty()) {
        scheme->setText(QStringLiteral("https"));
    }
    netloc->setText(data.value(QStringLiteral("netloc")).toString());
    path->setText(json_to_strings(data.value(QStringLiteral("components")).toArray()).join(QStringLiteral("/")));
    parameters->setPlainText(data.value(QStringLiteral("parameters")).toString());
    example->setText(data.value(QStringLiteral("example_url")).toString());
    match_subdomains->setChecked(data.value(QStringLiteral("match_subdomains")).toBool());
    keep_subdomains->setChecked(data.value(QStringLiteral("keep_matched_subdomains")).toBool());
    multiple->setChecked(data.value(QStringLiteral("can_produce_multiple_files")).toBool());
    associate->setChecked(data.value(QStringLiteral("should_be_associated_with_files")).toBool(true));

    const int index = type->findData(data.value(QStringLiteral("url_type")).toInt(0));
    if (index >= 0) {
        type->setCurrentIndex(index);
    }
}

QJsonObject HydrusUrlClassDialog::payload() const {
    QJsonObject value;
    value.insert(QStringLiteral("original"), data.value(QStringLiteral("name")).toString());
    value.insert(QStringLiteral("name"), name->text().trimmed());
    value.insert(QStringLiteral("url_type"), type->currentData().toInt());
    value.insert(QStringLiteral("preferred_scheme"), scheme->text().trimmed());
    value.insert(QStringLiteral("netloc"), netloc->text().trimmed());
    value.insert(QStringLiteral("path"), path->text().trimmed());
    value.insert(QStringLiteral("parameters"), parameters->toPlainText());
    value.insert(QStringLiteral("example_url"), example->text().trimmed());
    value.insert(QStringLiteral("match_subdomains"), match_subdomains->isChecked());
    value.insert(QStringLiteral("keep_matched_subdomains"), keep_subdomains->isChecked());
    value.insert(QStringLiteral("can_produce_multiple_files"), multiple->isChecked());
    value.insert(QStringLiteral("should_be_associated_with_files"), associate->isChecked());

    return value;
}

void HydrusUrlClassDialog::run_test() {
    const QUrl url(test->text().trimmed());

    if (!url.isValid() || url.host().isEmpty()) {
        result->setText(QStringLiteral("not a valid http(s) url"));
        return;
    }

    QString domain = netloc->text().trimmed().toLower();
    domain.remove(QStringLiteral("www."));
    QString host = url.host().toLower();
    host.remove(QStringLiteral("www."));

    if (host != domain
        && !(match_subdomains->isChecked() && host.endsWith(QStringLiteral(".") + domain))) {
        result->setText(QStringLiteral("domain does not match %1").arg(domain));
        return;
    }

    const QStringList expected = path->text().trimmed().split(QLatin1Char('/'), Qt::SkipEmptyParts);
    const QStringList actual = url.path().trimmed().split(QLatin1Char('/'), Qt::SkipEmptyParts);

    if (expected.size() != actual.size()) {
        result->setText(QStringLiteral("path has %1 segments; expected %2").arg(actual.size()).arg(expected.size()));
        return;
    }

    for (int index = 0; index < expected.size(); ++index) {
        const QString component = expected.at(index);

        if (component.startsWith(QLatin1Char('{')) && component.endsWith(QLatin1Char('}'))) {
            continue;
        }

        if (component != actual.at(index)) {
            result->setText(QStringLiteral("segment %1 does not match").arg(index + 1));
            return;
        }
    }

    result->setText(QStringLiteral("matches this url class"));
}
