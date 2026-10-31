#include "tag_relations_dialog.hpp"

// WS-15.1.3c1: tag relations dialog moved verbatim from bridge.cpp (move-only split).

#include "bridge.hpp"
#include "support/support.hpp"

#include <QComboBox>
#include <QDialogButtonBox>
#include <QFormLayout>
#include <QHBoxLayout>
#include <QHeaderView>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonValue>
#include <QLabel>
#include <QLineEdit>
#include <QPushButton>
#include <QSignalBlocker>
#include <QTabWidget>
#include <QTableWidget>
#include <QVBoxLayout>

HydrusTagRelationsDialog::HydrusTagRelationsDialog(HydrusMainWindow *main_window, const QJsonObject &state, QWidget *parent)
    : QDialog(parent), main(main_window) {
    setWindowTitle(QStringLiteral("tag siblings and parents"));
    setAttribute(Qt::WA_DeleteOnClose, true);
    resize(720, 520);

    auto *layout = new QVBoxLayout(this);

    auto *service_row = new QHBoxLayout();
    service_row->addWidget(new QLabel(QStringLiteral("service"), this));
    services = new QComboBox(this);
    service_row->addWidget(services, 1);
    layout->addLayout(service_row);

    auto *help = new QLabel(
        QStringLiteral("Siblings redirect a bad tag onto its preferred spelling. Parents describe a "
                       "child -> parent hierarchy. The left column is the tag being changed; the right "
                       "column is what it points to."),
        this);
    help->setWordWrap(true);
    layout->addWidget(help);

    tabs = new QTabWidget(this);

    auto make_table = [this]() {
        auto *table = new QTableWidget(this);
        table->setColumnCount(2);
        table->setHorizontalHeaderLabels({QStringLiteral("tag"), QStringLiteral("points to")});
        table->horizontalHeader()->setStretchLastSection(true);
        table->horizontalHeader()->setSectionResizeMode(0, QHeaderView::Stretch);
        table->setSelectionBehavior(QAbstractItemView::SelectRows);
        table->setSelectionMode(QAbstractItemView::SingleSelection);
        table->setEditTriggers(QAbstractItemView::NoEditTriggers);
        table->verticalHeader()->setVisible(false);
        return table;
    };

    siblings = make_table();
    parents = make_table();
    tabs->addTab(siblings, QStringLiteral("siblings"));
    tabs->addTab(parents, QStringLiteral("parents"));
    layout->addWidget(tabs, 1);

    auto *buttons = new QHBoxLayout();
    auto *add = new QPushButton(QStringLiteral("add pair..."), this);
    auto *remove = new QPushButton(QStringLiteral("remove"), this);
    auto *close_button = new QPushButton(QStringLiteral("close"), this);
    buttons->addWidget(add);
    buttons->addWidget(remove);
    buttons->addStretch(1);
    buttons->addWidget(close_button);
    layout->addLayout(buttons);

    connect(services, QOverload<int>::of(&QComboBox::currentIndexChanged), this, [this](int) {
        rebuild();
    });
    connect(add, &QPushButton::clicked, this, [this]() { add_relation(); });
    connect(remove, &QPushButton::clicked, this, [this]() { remove_selected(); });
    connect(close_button, &QPushButton::clicked, this, &QDialog::close);

    set_state(state);
}

void HydrusTagRelationsDialog::set_state(const QJsonObject &state) {
    const bool first_load = data.isEmpty();
    data = state;

    const QString previous = services->currentData().toString();
    QSignalBlocker blocker(services);
    services->clear();

    for (const QJsonValue &value : state.value(QStringLiteral("services")).toArray()) {
        const QJsonObject service = value.toObject();
        services->addItem(service.value(QStringLiteral("name")).toString(),
                          service.value(QStringLiteral("key")).toString());
    }

    if (services->count() == 0) {
        services->addItem(QStringLiteral("no local tag service"), QString());
    }

    if (!previous.isEmpty()) {
        const int index = services->findData(previous);

        if (index >= 0) {
            services->setCurrentIndex(index);
        }
    }

    blocker.unblock();

    if (first_load) {
        const QString tab = state.value(QStringLiteral("tab")).toString();
        tabs->setCurrentIndex(tab == QStringLiteral("parents") ? 1 : 0);
    }

    rebuild();
}

void HydrusTagRelationsDialog::rebuild() {
    siblings->setRowCount(0);
    parents->setRowCount(0);

    const QString key = services->currentData().toString();

    for (const QJsonValue &value : data.value(QStringLiteral("relations")).toArray()) {
        const QJsonObject relation = value.toObject();

        if (relation.value(QStringLiteral("service")).toString() != key) {
            continue;
        }

        const bool is_parent = relation.value(QStringLiteral("relation")).toString() == QStringLiteral("parents");
        QTableWidget *table = is_parent ? parents : siblings;
        const int row = table->rowCount();
        table->setRowCount(row + 1);

        auto *first = new QTableWidgetItem(relation.value(QStringLiteral("first")).toString());
        first->setData(Qt::UserRole, relation.value(QStringLiteral("first")).toString());
        auto *second = new QTableWidgetItem(relation.value(QStringLiteral("second")).toString());
        second->setData(Qt::UserRole, relation.value(QStringLiteral("second")).toString());

        table->setItem(row, 0, first);
        table->setItem(row, 1, second);
    }

    for (QTableWidget *table : {siblings, parents}) {
        table->resizeRowsToContents();
    }
}

QTableWidget *HydrusTagRelationsDialog::current_table() const {
    return tabs->currentIndex() == 1 ? parents : siblings;
}

QString HydrusTagRelationsDialog::current_relation() const {
    return tabs->currentIndex() == 1 ? QStringLiteral("parents") : QStringLiteral("siblings");
}

void HydrusTagRelationsDialog::add_relation() {
    const QString relation = current_relation();
    const bool is_parent = relation == QStringLiteral("parents");

    QDialog dialog(this);
    dialog.setWindowTitle(is_parent ? QStringLiteral("add tag parent") : QStringLiteral("add tag sibling"));
    auto *layout = new QVBoxLayout(&dialog);
    auto *form = new QFormLayout();

    auto *first = new QLineEdit(&dialog);
    auto *second = new QLineEdit(&dialog);
    first->setPlaceholderText(is_parent ? QStringLiteral("child tag") : QStringLiteral("bad tag"));
    second->setPlaceholderText(is_parent ? QStringLiteral("parent tag") : QStringLiteral("good tag"));
    form->addRow(is_parent ? QStringLiteral("child") : QStringLiteral("bad tag"), first);
    form->addRow(is_parent ? QStringLiteral("parent") : QStringLiteral("good tag"), second);
    layout->addLayout(form);

    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, &dialog);
    connect(buttons, &QDialogButtonBox::accepted, &dialog, &QDialog::accept);
    QObject::connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);
    layout->addWidget(buttons);

    if (dialog.exec() != QDialog::Accepted) {
        return;
    }

    if (first->text().trimmed().isEmpty() || second->text().trimmed().isEmpty()) {
        return;
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("relation"), relation);
    payload.insert(QStringLiteral("service"), services->currentData().toString());
    payload.insert(QStringLiteral("first"), first->text().trimmed());
    payload.insert(QStringLiteral("second"), second->text().trimmed());

    main->notify(QStringLiteral("tag_relations_add"), QString(), json_payload(payload));
}

void HydrusTagRelationsDialog::remove_selected() {
    QTableWidget *table = current_table();
    const int row = table->currentRow();

    if (row < 0 || table->item(row, 0) == nullptr || table->item(row, 1) == nullptr) {
        return;
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("relation"), current_relation());
    payload.insert(QStringLiteral("service"), services->currentData().toString());
    payload.insert(QStringLiteral("first"), table->item(row, 0)->data(Qt::UserRole).toString());
    payload.insert(QStringLiteral("second"), table->item(row, 1)->data(Qt::UserRole).toString());

    main->notify(QStringLiteral("tag_relations_remove"), QString(), json_payload(payload));
}

void HydrusMainWindow::show_tag_relations(const QJsonObject &state) {
    if (tag_relations_dialog == nullptr) {
        tag_relations_dialog = new HydrusTagRelationsDialog(this, state, this);
        connect(tag_relations_dialog, &QObject::destroyed, this, [this]() {
            tag_relations_dialog = nullptr;
        });
        tag_relations_dialog->show();
    } else {
        tag_relations_dialog->set_state(state);
    }

    tag_relations_dialog->raise();
    tag_relations_dialog->activateWindow();
}
