#include "network.hpp"

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

namespace {

class HydrusUrlClassDialog : public QDialog {
public:
    HydrusUrlClassDialog(const QJsonObject &entry, const QJsonArray &types, QWidget *parent = nullptr)
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

    QJsonObject payload() const {
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

private:
    void run_test() {
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

    QJsonObject data;
    QLineEdit *name = nullptr;
    QComboBox *type = nullptr;
    QLineEdit *scheme = nullptr;
    QLineEdit *netloc = nullptr;
    QLineEdit *path = nullptr;
    QPlainTextEdit *parameters = nullptr;
    QLineEdit *example = nullptr;
    QCheckBox *match_subdomains = nullptr;
    QCheckBox *keep_subdomains = nullptr;
    QCheckBox *multiple = nullptr;
    QCheckBox *associate = nullptr;
    QLineEdit *test = nullptr;
    QLabel *result = nullptr;
};

} // namespace


void HydrusNetworkDialog::add_header() {
    QDialog dialog(this);
    dialog.setWindowTitle(QStringLiteral("add custom header"));
    auto *layout = new QVBoxLayout(&dialog);
    auto *form = new QFormLayout();
    auto *name = new QLineEdit(&dialog);
    auto *value = new QLineEdit(&dialog);
    form->addRow(QStringLiteral("name"), name);
    form->addRow(QStringLiteral("value"), value);
    layout->addLayout(form);
    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, &dialog);
    connect(buttons, &QDialogButtonBox::accepted, &dialog, &QDialog::accept);
    QObject::connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);
    layout->addWidget(buttons);

    if (dialog.exec() != QDialog::Accepted || name->text().trimmed().isEmpty()) {
        return;
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("name"), name->text().trimmed());
    payload.insert(QStringLiteral("value"), value->text());

    main->notify(QStringLiteral("network_header_add"), QString(), json_payload(payload));
}

void HydrusNetworkDialog::remove_header() {
    const int row = headers->currentRow();

    if (row < 0 || headers->item(row, 0) == nullptr) {
        return;
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("name"), headers->item(row, 0)->text());

    main->notify(QStringLiteral("network_header_remove"), QString(), json_payload(payload));
}

void HydrusNetworkDialog::add_url_class() {
    HydrusUrlClassDialog dialog(QJsonObject(), data.value(QStringLiteral("url_types")).toArray(), this);

    if (dialog.exec() != QDialog::Accepted) {
        return;
    }

    QJsonObject payload = dialog.payload();

    if (payload.value(QStringLiteral("netloc")).toString().isEmpty()) {
        return;
    }

    main->notify(QStringLiteral("network_url_class_add"), QString(), json_payload(payload));
}

void HydrusNetworkDialog::edit_url_class() {
    const int row = url_classes->currentRow();
    const QJsonArray classes = data.value(QStringLiteral("url_classes")).toArray();

    if (row < 0 || row >= classes.size()) {
        return;
    }

    HydrusUrlClassDialog dialog(
        classes.at(row).toObject(),
        data.value(QStringLiteral("url_types")).toArray(),
        this);

    if (dialog.exec() != QDialog::Accepted) {
        return;
    }

    main->notify(
        QStringLiteral("network_url_class_save"),
        QString(),
        json_payload(dialog.payload()));
}

void HydrusNetworkDialog::remove_url_class() {
    const int row = url_classes->currentRow();

    if (row < 0 || url_classes->item(row, 0) == nullptr) {
        return;
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("name"), url_classes->item(row, 0)->text());

    main->notify(QStringLiteral("network_url_class_remove"), QString(), json_payload(payload));
}

void HydrusNetworkDialog::add_pool() {
    QDialog dialog(this);
    dialog.setWindowTitle(QStringLiteral("domain download pool"));
    auto *layout = new QVBoxLayout(&dialog);
    auto *form = new QFormLayout();
    auto *domain = new QLineEdit(&dialog);
    domain->setPlaceholderText(QStringLiteral("example.com"));
    auto *limit = new QSpinBox(&dialog);
    limit->setRange(1, 64);
    limit->setValue(2);
    form->addRow(QStringLiteral("domain"), domain);
    form->addRow(QStringLiteral("simultaneous downloads"), limit);
    layout->addLayout(form);
    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, &dialog);
    connect(buttons, &QDialogButtonBox::accepted, &dialog, &QDialog::accept);
    QObject::connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);
    layout->addWidget(buttons);

    if (dialog.exec() != QDialog::Accepted || domain->text().trimmed().isEmpty()) {
        return;
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("domain"), domain->text().trimmed());
    payload.insert(QStringLiteral("limit"), limit->value());
    main->notify(QStringLiteral("network_pool_set"), QString(), json_payload(payload));
}

void HydrusNetworkDialog::remove_pool() {
    const int row = pools->currentRow();

    if (row < 0 || pools->item(row, 0) == nullptr) {
        return;
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("domain"), pools->item(row, 0)->data(Qt::UserRole).toString());
    main->notify(QStringLiteral("network_pool_remove"), QString(), json_payload(payload));
}

void HydrusNetworkDialog::add_bandwidth_rule() {
    QDialog dialog(this);
    dialog.setWindowTitle(QStringLiteral("bandwidth rule"));
    auto *layout = new QVBoxLayout(&dialog);
    auto *form = new QFormLayout();
    auto *context_type = new QComboBox(&dialog);
    context_type->addItem(QStringLiteral("global"), QStringLiteral("global"));
    context_type->addItem(QStringLiteral("domain"), QStringLiteral("domain"));
    context_type->addItem(QStringLiteral("hydrus"), QStringLiteral("hydrus"));
    auto *context = new QLineEdit(&dialog);
    context->setPlaceholderText(QStringLiteral("domain name (domain rules only)"));
    auto *bandwidth_type = new QComboBox(&dialog);
    bandwidth_type->addItem(QStringLiteral("requests"), QStringLiteral("requests"));
    bandwidth_type->addItem(QStringLiteral("data"), QStringLiteral("data"));
    auto *period = new QSpinBox(&dialog);
    period->setRange(0, 30 * 86400);
    period->setValue(1);
    period->setSpecialValueText(QStringLiteral("calendar month"));
    auto *maximum = new QSpinBox(&dialog);
    maximum->setRange(0, 2'000'000'000);
    maximum->setValue(1);
    auto *hint = new QLabel(QStringLiteral("Data maximums are in MB; request maximums are counts."), &dialog);
    hint->setWordWrap(true);
    form->addRow(QStringLiteral("context"), context_type);
    form->addRow(QStringLiteral("domain"), context);
    form->addRow(QStringLiteral("type"), bandwidth_type);
    form->addRow(QStringLiteral("period (seconds)"), period);
    form->addRow(QStringLiteral("maximum"), maximum);
    layout->addLayout(form);
    layout->addWidget(hint);
    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, &dialog);
    connect(buttons, &QDialogButtonBox::accepted, &dialog, &QDialog::accept);
    QObject::connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);
    layout->addWidget(buttons);

    connect(bandwidth_type, QOverload<int>::of(&QComboBox::currentIndexChanged), &dialog, [maximum, bandwidth_type](int) {
        if (bandwidth_type->currentData().toString() == QStringLiteral("data")) {
            maximum->setRange(0, 2'000'000);
            maximum->setValue(1024);
        } else {
            maximum->setRange(0, 2'000'000'000);
            maximum->setValue(1);
        }
    });

    if (dialog.exec() != QDialog::Accepted) {
        return;
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("context_type"), context_type->currentData().toString());
    payload.insert(QStringLiteral("context"), context->text().trimmed());
    payload.insert(QStringLiteral("bandwidth_type"), bandwidth_type->currentData().toString());

    if (period->value() <= 0) {
        payload.insert(QStringLiteral("time_delta"), QJsonValue::Null);
    } else {
        payload.insert(QStringLiteral("time_delta"), period->value());
    }

    payload.insert(
        QStringLiteral("max_allowed"),
        bandwidth_type->currentData().toString() == QStringLiteral("data")
            ? static_cast<qint64>(maximum->value()) * 1024 * 1024
            : static_cast<qint64>(maximum->value()));

    main->notify(QStringLiteral("network_rule_set"), QString(), json_payload(payload));
}

void HydrusNetworkDialog::remove_bandwidth_rule() {
    const int row = bandwidth_rules->currentRow();

    if (row < 0 || bandwidth_rules->item(row, 0) == nullptr) {
        return;
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("context_type"), bandwidth_rules->item(row, 0)->data(Qt::UserRole).toString());
    payload.insert(QStringLiteral("context"), bandwidth_rules->item(row, 1)->data(Qt::UserRole).toString());
    payload.insert(QStringLiteral("bandwidth_type"), bandwidth_rules->item(row, 2)->data(Qt::UserRole).toString());

    main->notify(QStringLiteral("network_rule_remove"), QString(), json_payload(payload));
}
