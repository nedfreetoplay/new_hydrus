#include "duplicates_page.hpp"

// WS-15.1.2c: moved verbatim from bridge.cpp (move-only split).

#include "bridge.hpp"
#include "support/support.hpp"

#include <QAction>
#include <QApplication>
#include <QCheckBox>
#include <QClipboard>
#include <QComboBox>
#include <QDateTime>
#include <QFont>
#include <QFrame>
#include <QHBoxLayout>
#include <QHeaderView>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonValue>
#include <QLabel>
#include <QLineEdit>
#include <QListWidget>
#include <QMenu>
#include <QMessageBox>
#include <QPalette>
#include <QPlainTextEdit>
#include <QProgressBar>
#include <QPushButton>
#include <QRegularExpression>
#include <QScrollArea>
#include <QSignalBlocker>
#include <QSizePolicy>
#include <QSpinBox>
#include <QSplitter>
#include <QStyle>
#include <QTableWidget>
#include <QTabWidget>
#include <QTimer>
#include <QToolButton>
#include <QVBoxLayout>

void HydrusDuplicatesPage::build_media_panel(
    QSplitter *page_splitter,
    QScrollArea *sidebar_scroll,
    QWidget *sidebar,
    QVBoxLayout *sidebar_layout,
    QHBoxLayout *outer,
    const QJsonObject &state) {
    auto *right = new QWidget(page_splitter);
    auto *layout = new QVBoxLayout(right);
    layout->setContentsMargins(6, 6, 6, 6);
    layout->setSpacing(6);
    page_splitter->addWidget(sidebar_scroll);
    page_splitter->addWidget(right);
    page_splitter->setStretchFactor(0, 0);
    page_splitter->setStretchFactor(1, 1);
    page_splitter->setSizes({440, 872});
    outer->addWidget(page_splitter);

    auto *header = new QHBoxLayout();
    status = new QLabel(this);
    distance = new QLabel(this);
    auto *refresh_button = new QPushButton(QStringLiteral("refresh"), this);
    refresh_button->setObjectName(QStringLiteral("refresh_duplicates"));
    refresh = refresh_button;
    delete_both = new QPushButton(QStringLiteral("delete both"), this);
    rules = new QPushButton(QStringLiteral("auto-resolution..."), this);
    merge_options = new QPushButton(QStringLiteral("merge options..."), this);
    auto *review = new QPushButton(QStringLiteral("review pending..."), this);
    rules->hide();
    merge_options->hide();
    review->hide();
    batch = new QSpinBox(this);
    batch->setRange(5, 1024);
    batch->setToolTip(QStringLiteral("maximum pairs prepared in one batch"));
    next_batch = new QPushButton(QStringLiteral("next batch"), this);
    status->setMinimumWidth(0);
    status->setSizePolicy(QSizePolicy::Ignored, QSizePolicy::Preferred);
    header->addWidget(status, 1);
    header->addWidget(distance);
    header->addWidget(new QLabel(QStringLiteral("batch:"), this));
    header->addWidget(batch);
    header->addWidget(next_batch);
    group_actions = new QPushButton(QStringLiteral("actions..."), this);
    group_actions->setToolTip(QStringLiteral("reset potentials, clear false positives, or dissolve duplicate groups"));
    group_actions->hide();
    header->addWidget(refresh_button);
    header->addWidget(delete_both);
    layout->addLayout(header);

    media_status = new QLabel(right);
    media_status->setWordWrap(true);
    layout->addWidget(media_status);

    grid = new HydrusThumbnailGrid(main, this);
    layout->addWidget(grid, 1);

    auto *bottom = new QHBoxLayout();
    same = new QPushButton(QStringLiteral("they are the same"), this);
    alternates = new QPushButton(QStringLiteral("they are related alternates"), this);
    custom_action = new QPushButton(QStringLiteral("custom action..."), this);
    not_duplicates = new QPushButton(QStringLiteral("not duplicates"), this);
    skip = new QPushButton(QStringLiteral("skip"), this);
    bottom->addStretch(1);
    bottom->addWidget(same);
    bottom->addWidget(alternates);
    bottom->addWidget(custom_action);
    bottom->addWidget(not_duplicates);
    bottom->addWidget(skip);
    bottom->addStretch(1);
    layout->addLayout(bottom);

    sidebar_scroll->setMinimumWidth(440);
    sidebar->setMinimumHeight(sidebar_layout->minimumSize().height());
    page_splitter->setSizes({440, 872});

    connect(same, &QPushButton::clicked, this, [this]() { act(QStringLiteral("duplicates_same")); });
    connect(alternates, &QPushButton::clicked, this, [this]() { act(QStringLiteral("duplicates_alternates")); });
    connect(custom_action, &QPushButton::clicked, this, [this]() { act(QStringLiteral("duplicates_custom_action")); });
    connect(not_duplicates, &QPushButton::clicked, this, [this]() { act(QStringLiteral("duplicates_false")); });
    connect(skip, &QPushButton::clicked, this, [this]() { act(QStringLiteral("duplicates_skip")); });
    connect(refresh_button, &QPushButton::clicked, this, [this]() { act(QStringLiteral("duplicates_refresh")); });
    connect(next_batch, &QPushButton::clicked, this, [this]() { act(QStringLiteral("duplicates_next_batch")); });
    connect(batch, &QSpinBox::valueChanged, this, [this](int value) {
        main->notify(QStringLiteral("duplicates_batch_size"), id, QString::number(value));
    });
    connect(rules, &QPushButton::clicked, this, [this]() {
        main->notify(QStringLiteral("duplicates_rules"));
    });
    connect(merge_options, &QPushButton::clicked, this, [this]() {
        main->notify(QStringLiteral("duplicates_merge_options_dialog"));
    });
    connect(review, &QPushButton::clicked, this, [this]() {
        main->notify(QStringLiteral("duplicates_review"));
    });
    connect(group_actions, &QPushButton::clicked, this, [this]() {
        show_group_actions(QStringLiteral("both"), group_actions->mapToGlobal(QPoint(0, group_actions->height())));
    });
    connect(delete_both, &QPushButton::clicked, this, [this]() {
        if (QMessageBox::question(this, QStringLiteral("delete both"), QStringLiteral("Move both files to the trash?")) == QMessageBox::Yes) {
            act(QStringLiteral("duplicates_delete_both"));
        }
    });

    connect(dupe_search_type, &QComboBox::currentIndexChanged, this, [this]() {
        update_search_controls();
        notify_search_context();
    });
    connect(search_location_1, &QComboBox::currentTextChanged, this, [this]() { notify_search_context(); });
    connect(search_location_2, &QComboBox::currentTextChanged, this, [this]() { notify_search_context(); });
    connect(search_query_1, &QLineEdit::editingFinished, this, [this]() { notify_search_context(); });
    connect(search_query_2, &QLineEdit::editingFinished, this, [this]() { notify_search_context(); });
    connect(pixel_dupes, &QComboBox::currentIndexChanged, this, [this]() {
        update_search_controls();
        notify_search_context();
    });
    connect(search_distance, &QSpinBox::valueChanged, this, [this]() { notify_search_context(); });
    connect(sort_type, &QComboBox::currentIndexChanged, this, [this]() {
        update_search_controls();
        notify_search_context();
    });
    connect(sort_ascending, &QComboBox::currentIndexChanged, this, [this]() { notify_search_context(); });

    update_search_controls();

    update_state(state);

    if (qEnvironmentVariableIsSet("HYDRUS_QT_DUPLICATES_CUSTOM_ACTION")) {
        QTimer::singleShot(150, this, [this]() { act(QStringLiteral("duplicates_custom_action")); });
    }

    if (qEnvironmentVariableIsSet("HYDRUS_QT_DUPLICATES_ACTIONS_MENU")) {
        QTimer::singleShot(150, this, [this]() {
            show_group_actions(QStringLiteral("both"), mapToGlobal(QPoint(40, 40)));
        });
    }
}

void HydrusDuplicatesPage::notify_search_context() {
    const QString search_type = dupe_search_type->currentData().toString();

    if (search_type != QStringLiteral("different_searches")) {
        const QSignalBlocker blocker(search_location_2);
        search_location_2->setCurrentText(search_location_1->currentText());
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("query_1"), search_query_1->text());
    payload.insert(QStringLiteral("location_1"), search_location_1->currentText());
    payload.insert(QStringLiteral("query_2"), search_query_2->text());
    payload.insert(QStringLiteral("location_2"), search_location_2->currentText());
    payload.insert(QStringLiteral("dupe_search_type"), search_type);
    payload.insert(QStringLiteral("pixel_dupes"), pixel_dupes->currentData().toString());
    payload.insert(QStringLiteral("max_distance"), search_distance->value());
    payload.insert(QStringLiteral("sort_type"), sort_type->currentData().toString());
    payload.insert(QStringLiteral("sort_ascending"), sort_ascending->currentData().toBool());

    main->notify(QStringLiteral("duplicates_search_context"), id, json_payload(payload));
}

void HydrusDuplicatesPage::update_search_controls() {
    const bool different_searches = dupe_search_type->currentData().toString() == QStringLiteral("different_searches");

    search_row_2_label->setVisible(different_searches);
    search_query_2->setVisible(different_searches);
    search_location_2->setVisible(different_searches);

    if (search_query_2->parentWidget() != nullptr) {
        search_query_2->parentWidget()->setVisible(different_searches);
    }

    search_distance->setEnabled(pixel_dupes->currentData().toString() != QStringLiteral("required"));

    const QString sort = sort_type->currentData().toString();
    const bool random = sort == QStringLiteral("random");
    const bool current_asc = sort_ascending->currentData().toBool();

    sort_ascending->setVisible(!random);

    if (random) {
        return;
    }

    QString first;
    QString second;

    if (sort == QStringLiteral("similarity")) {
        first = QStringLiteral("most similar first");
        second = QStringLiteral("least similar first");
    } else {
        first = QStringLiteral("largest first");
        second = QStringLiteral("smallest first");
    }

    const QSignalBlocker blocker(sort_ascending);
    sort_ascending->setItemText(0, first);
    sort_ascending->setItemText(1, second);

    const int index = sort_ascending->findData(current_asc);

    if (index >= 0) {
        sort_ascending->setCurrentIndex(index);
    }
}

const QString &HydrusDuplicatesPage::page_id() const {
    return id;
}

void HydrusDuplicatesPage::show_group_actions(const QString &target, const QPoint &global_pos) {
    QMenu menu(this);
    const QString target_label = target == QStringLiteral("a")
        ? QStringLiteral("file A")
        : target == QStringLiteral("b") ? QStringLiteral("file B") : QStringLiteral("both files");

    auto add = [&menu](const QString &label, const QString &kind, const QString &tooltip) {
        QAction *action = menu.addAction(label);
        action->setData(kind);
        action->setToolTip(tooltip);
    };

    add(QStringLiteral("remove all potentials"),
        QStringLiteral("remove_potentials"),
        QStringLiteral("Stop these files from appearing in the duplicate filter unless new potentials are found."));

    add(QStringLiteral("reset potential search"),
        QStringLiteral("reset_potential_search"),
        QStringLiteral("Search these files for potential duplicates again. Existing pairs are kept."));

    menu.addSeparator();

    add(QStringLiteral("clear all false positives"),
        QStringLiteral("clear_false_positives"),
        QStringLiteral("Remove every false-positive relation for these files and their alternates."));

    add(QStringLiteral("clear internal false positives"),
        QStringLiteral("clear_internal_false_positives"),
        QStringLiteral("Remove false-positive relations between the selected files."));

    menu.addSeparator();

    add(QStringLiteral("remove from duplicate group"),
        QStringLiteral("remove_from_duplicate_group"),
        QStringLiteral("Pull these files out of their duplicate groups."));

    add(QStringLiteral("remove from alternates group"),
        QStringLiteral("remove_from_alternate_group"),
        QStringLiteral("Pull these files out of their alternates groups."));

    add(QStringLiteral("dissolve duplicate group"),
        QStringLiteral("dissolve_duplicate_group"),
        QStringLiteral("Remove the whole duplicate groups and their alternates relations."));

    add(QStringLiteral("dissolve alternates group"),
        QStringLiteral("dissolve_alternate_group"),
        QStringLiteral("Remove every duplicate and alternates relation for the whole alternates groups."));

    QAction *chosen = menu.exec(global_pos);

    if (chosen == nullptr) {
        return;
    }

    const QString kind = chosen->data().toString();

    if (kind.isEmpty()) {
        return;
    }

    const QString warning = QStringLiteral("%1 for %2?\n\nThis can throw away previous duplicate decisions and cannot be undone.")
        .arg(chosen->text(), target_label);

    if (QMessageBox::question(this, QStringLiteral("duplicate action"), warning) != QMessageBox::Yes) {
        return;
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("kind"), kind);
    payload.insert(QStringLiteral("target"), target);
    main->notify(QStringLiteral("duplicates_group_action"), id, json_payload(payload));
}

void HydrusDuplicatesPage::act(const QString &action) {
    main->notify(action, id);
}

void HydrusDuplicatesPage::update_state(const QJsonObject &state) {
    const int index = state.value(QStringLiteral("index")).toInt();
    const int total = state.value(QStringLiteral("total")).toInt();
    const QString page_status = state.value(QStringLiteral("status")).toString();

    status->setText(total == 0
        ? page_status
        : QStringLiteral("%1  -  %2/%3").arg(page_status).arg(index).arg(total));

    {
        const QSignalBlocker blocker(batch);
        batch->setValue(state.value(QStringLiteral("batch_size")).toInt(100));
    }

    const QJsonObject search = state.value(QStringLiteral("search")).toObject();

    if (!search.isEmpty()) {
        auto set_combo_data = [](QComboBox *combo, const QString &value) {
            const int index = combo->findData(value);

            if (index >= 0) {
                const QSignalBlocker blocker(combo);
                combo->setCurrentIndex(index);
            }
        };

        const QSignalBlocker query_1_blocker(search_query_1);
        const QSignalBlocker query_2_blocker(search_query_2);
        const QSignalBlocker distance_blocker(search_distance);

        const QString query_1 = search.value(QStringLiteral("query_1")).toString();
        const QString query_2 = search.value(QStringLiteral("query_2")).toString();
        search_query_1->setText(query_1.isEmpty() ? QStringLiteral("system:everything")
                                                  : query_1);
        search_query_2->setText(query_2.isEmpty() ? QStringLiteral("system:everything")
                                                  : query_2);
        search_distance->setValue(search.value(QStringLiteral("max_distance")).toInt(4));
        set_combo_data(dupe_search_type, search.value(QStringLiteral("dupe_search_type")).toString());
        set_combo_data(pixel_dupes, search.value(QStringLiteral("pixel_dupes")).toString());

        {
            const QSignalBlocker sort_type_blocker(sort_type);
            const QSignalBlocker sort_asc_blocker(sort_ascending);

            set_combo_data(sort_type, search.value(QStringLiteral("sort_type")).toString());
            const int sort_index = sort_ascending->findData(search.value(QStringLiteral("sort_ascending")).toBool());

            if (sort_index >= 0) {
                sort_ascending->setCurrentIndex(sort_index);
            }
        }

        {
            const QSignalBlocker location_1_blocker(search_location_1);
            const QSignalBlocker location_2_blocker(search_location_2);
            const QString location_1 = search.value(QStringLiteral("location_1")).toString();
            const QString location_2 = search.value(QStringLiteral("location_2")).toString();

            if (!location_1.isEmpty()) {
                search_location_1->setCurrentText(location_1);
            }

            if (!location_2.isEmpty()) {
                search_location_2->setCurrentText(location_2);
            }
        }

        const int matched = search.value(QStringLiteral("matched")).toInt();
        const int searched = state.value(QStringLiteral("searched")).toInt();

        if (searched == 0) {
            search_summary->setText(
                QStringLiteral("no potential pairs in this file domain!"));
        } else {
            search_summary->setText(QStringLiteral("%1 of %2 pairs match  -  %3")
                .arg(matched)
                .arg(searched)
                .arg(search.value(QStringLiteral("summary")).toString()));
        }
    }

    update_search_controls();

    const QJsonValue pair_value = state.value(QStringLiteral("pair"));

    if (!pair_value.isObject()) {
        distance->setText(QString());

        for (QPushButton *button : {same, not_duplicates, skip, delete_both}) {
            button->setEnabled(false);
        }
    } else {
        for (QPushButton *button : {same, not_duplicates, skip, delete_both}) {
            button->setEnabled(true);
        }

        const QJsonObject pair = pair_value.toObject();
        distance->setText(QStringLiteral("distance %1").arg(pair.value(QStringLiteral("distance")).toInt()));
    }

    const QJsonArray media = state.value(QStringLiteral("media")).toArray();

    if (grid != nullptr) {
        grid->set_media(media);
    }

    if (media_status != nullptr) {
        if (media.isEmpty()) {
            media_status->setText(state.value(QStringLiteral("media_empty_status")).toString());
        } else {
            media_status->setText(state.value(QStringLiteral("media_count_status")).toString());
        }
    }

    update_auto_resolution_rules(state);
}

void HydrusDuplicatesPage::update_auto_resolution_rules(const QJsonObject &state) {
    if (auto_rules_table == nullptr) {
        return;
    }

    QJsonArray rules = state.value(QStringLiteral("auto_rules")).toArray();
    if (rules.isEmpty()) {
        rules = state.value(QStringLiteral("rules")).toArray();
    }
    auto_rules_table->setRowCount(rules.size());

    for (int index = 0; index < rules.size(); ++index) {
        const QJsonObject rule = rules.at(index).toObject();

        for (int column = 0; column < 3; ++column) {
            if (auto_rules_table->item(index, column) == nullptr) {
                auto_rules_table->setItem(index, column, new QTableWidgetItem());
            }
        }

        auto_rules_table->item(index, 0)->setText(rule.value(QStringLiteral("name")).toString());

        QString progress = rule.value(QStringLiteral("search_summary")).toString();
        auto_rules_table->item(index, 1)->setText(progress);

        QString status = rule.value(QStringLiteral("paused")).toBool()
                             ? QStringLiteral("paused")
                             : QStringLiteral("running");
        auto_rules_table->item(index, 2)->setText(status);
    }

    if (auto_rules_status != nullptr) {
        if (rules.isEmpty()) {
            auto_rules_status->setText(QStringLiteral("no auto-resolution rules yet"));
            auto_rules_status->setVisible(true);
        } else {
            auto_rules_status->setVisible(false);
        }
    }

    auto_rules_table->resizeRowsToContents();
}