#include "downloader_page.hpp"

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

void HydrusDownloaderPage::build_gallery_panel(QVBoxLayout *main_layout) {
    const bool gallery = kind == QStringLiteral("gallery");
    if (gallery) {
        status_top = new QLabel(main_content);
        status_top->setTextFormat(Qt::PlainText);
        main_layout->addWidget(status_top);

        status_bottom = new QLabel(main_content);
        status_bottom->setTextFormat(Qt::PlainText);
        main_layout->addWidget(status_bottom);

        queue = new QTableWidget(main_content);
        queue->setColumnCount(7);
        queue->setHorizontalHeaderLabels({QStringLiteral("query"),
                                          QStringLiteral("source"),
                                          QStringLiteral("files status"),
                                          QStringLiteral("search status"),
                                          QStringLiteral("status"),
                                          QStringLiteral("items"),
                                          QStringLiteral("added")});
        queue->verticalHeader()->setVisible(false);
        queue->setSelectionBehavior(QAbstractItemView::SelectRows);
        queue->setSelectionMode(QAbstractItemView::SingleSelection);
        queue->setEditTriggers(QAbstractItemView::NoEditTriggers);
        queue->setHorizontalScrollMode(QAbstractItemView::ScrollPerPixel);
        queue->setFixedHeight(128);

        queue->setSortingEnabled(true);
        queue->setTextElideMode(Qt::ElideRight);
        queue->horizontalHeader()->setTextElideMode(Qt::ElideRight);
        queue->horizontalHeader()->setFixedHeight(24);

        const int widths[] = {56, 78, 30, 30, 56, 64, 56};

        for (int index = 0; index < 7; ++index) {
            queue->setColumnWidth(index, widths[index]);
            queue->horizontalHeader()->setSectionResizeMode(index, QHeaderView::Fixed);
        }

        queue->sortByColumn(0, Qt::AscendingOrder);

        main_layout->addWidget(queue);

        auto *icon_row = new QWidget(main_content);
        auto *icon_layout = new QHBoxLayout(icon_row);
        icon_layout->setContentsMargins(0, 0, 0, 0);
        icon_layout->setSpacing(2);
        icon_layout->addStretch(1);

        auto *highlight_button = make_row_icon(icon_row, QStringLiteral("highlight"), QStringLiteral("highlight"));
        auto *clear_highlight_button =
            make_row_icon(icon_row, QStringLiteral("unhighlight"), QStringLiteral("clear highlight"));
        files_pause = make_row_icon(icon_row, QStringLiteral("pause"), QStringLiteral("pause/play files"));
        search_pause = make_row_icon(icon_row, QStringLiteral("pause"), QStringLiteral("pause/play search"));
        auto *retry_button = make_row_icon(icon_row, QStringLiteral("retry"), QStringLiteral("retry commands"));
        auto *remove_button = make_row_icon(icon_row, QStringLiteral("trash"), QStringLiteral("remove selected"));

        for (QToolButton *button : {highlight_button,
                                    clear_highlight_button,
                                    files_pause,
                                    search_pause,
                                    retry_button,
                                    remove_button}) {
            icon_layout->addWidget(button, 0);
        }

        main_layout->addWidget(icon_row);

        connect(highlight_button, &QToolButton::clicked, this, [this]() {
            main->notify(QStringLiteral("downloader_open_results"), id);
        });
        connect(clear_highlight_button, &QToolButton::clicked, this, [this]() {
            main->notify(QStringLiteral("downloader_clear"), id);
        });
        connect(files_pause, &QToolButton::clicked, this, [this]() {
            main->notify(QStringLiteral("downloader_pause"), id);
        });
        connect(search_pause, &QToolButton::clicked, this, [this]() {
            main->notify(QStringLiteral("downloader_pause"), id);
        });
        connect(retry_button, &QToolButton::clicked, this, [this]() { start(); });
        connect(remove_button, &QToolButton::clicked, this, [this]() {
            main->notify(QStringLiteral("downloader_clear"), id);
        });
        connect(queue, &QTableWidget::cellDoubleClicked, this, [this](int, int) { start(); });

        query_row = new QWidget(main_content);
        auto *query_layout = new QHBoxLayout(query_row);
        query_layout->setContentsMargins(0, 0, 0, 0);
        query_layout->setSpacing(2);

        query = new QLineEdit(query_row);
        query->setMinimumHeight(24);
        query->setPlaceholderText(QStringLiteral("unknown downloader"));
        query_layout->addWidget(query, 1);

        auto *paste_button = make_row_icon(query_row, QStringLiteral("paste"), QStringLiteral("paste"));
        paste_button->setFixedSize(28, 24);
        query_layout->addWidget(paste_button, 0);

        auto *query_cog = make_row_icon(query_row, QStringLiteral("gear"), QStringLiteral("downloader options"));
        query_layout->addWidget(query_cog, 0);
        main_layout->addWidget(query_row);

        connect(query, &QLineEdit::returnPressed, this, [this]() { start(); });
        connect(paste_button, &QToolButton::clicked, this, [this]() {
            const QString text = QApplication::clipboard()->text().trimmed();

            if (!text.isEmpty()) {
                query->setText(text.split('\n').first().trimmed());
                start();
            }
        });

        connect(query_cog, &QToolButton::clicked, this, [this, query_cog]() {
            auto *menu = new QMenu(this);
            menu->setAttribute(Qt::WA_DeleteOnClose);
            menu->addAction(QStringLiteral("start query"), this, [this]() { start(); });
            menu->addAction(QStringLiteral("clear finished"), this, [this]() {
                main->notify(QStringLiteral("downloader_clear"), id);
            });
            menu->addAction(QStringLiteral("file seed cache..."), this, [this]() {
                main->notify(QStringLiteral("file_seed_cache"), id);
            });
            menu->addAction(QStringLiteral("gallery log..."), this, [this]() {
                main->notify(QStringLiteral("gallery_seed_log"), id);
            });
            menu->popup(query_cog->mapToGlobal(QPoint(0, query_cog->height())));
        });

        gug_button = new QPushButton(QStringLiteral("no downloader set"), main_content);
        gug_button->setMinimumHeight(24);
        main_layout->addWidget(gug_button);

        site = new QComboBox(main_content);
        site->hide();

        connect(gug_button, &QPushButton::clicked, this, [this]() {
            auto *menu = new QMenu(this);
            menu->setAttribute(Qt::WA_DeleteOnClose);
            auto *none = menu->addAction(QStringLiteral("no downloader set"));

            connect(none, &QAction::triggered, this, [this]() {
                site->setCurrentIndex(0);
                gug_button->setText(QStringLiteral("no downloader set"));
                url->clear();
                main->notify(QStringLiteral("downloader_set_site"), id, QString());
            });

            for (int index = 1; index < site->count(); ++index) {
                const QString title = site->itemText(index);
                auto *action = menu->addAction(title);

                connect(action, &QAction::triggered, this, [this, index, title]() {
                    site->setCurrentIndex(index);
                    gug_button->setText(title);
                    url->setText(site->itemData(index, Qt::UserRole + 1).toString());
                    main->notify(
                        QStringLiteral("downloader_set_site"),
                        id,
                        site->itemData(index).toString());
                });
            }

            menu->popup(gug_button->mapToGlobal(QPoint(0, gug_button->height())));
        });

        file_limit_row = new QWidget(main_content);
        auto *limit_layout = new QHBoxLayout(file_limit_row);
        limit_layout->setContentsMargins(0, 0, 0, 0);
        limit_layout->setSpacing(4);

        auto *limit_label = new QLabel(QStringLiteral("stop after this many files:"), file_limit_row);
        limit_layout->addWidget(limit_label, 0);

        file_limit = new QSpinBox(file_limit_row);
        file_limit->setRange(1, 10000000);
        file_limit->setValue(2000);
        file_limit->setFixedHeight(20);
        limit_layout->addWidget(file_limit, 1);

        no_limit = new QCheckBox(QStringLiteral("no limit"), file_limit_row);
        limit_layout->addWidget(no_limit, 0);
        main_layout->addWidget(file_limit_row);
    }
}

void HydrusDownloaderPage::build_watcher_panel(QVBoxLayout *main_layout) {
    const bool watcher = kind == QStringLiteral("watcher");
    if (watcher) {
        // Python's SidebarImporterMultipleWatcher: status labels, the watchers
        // table with its action icons, the url input and the options buttons.
        // The imports/search panels live in "highlighted watcher" below.
        status_top = new QLabel(main_content);
        status_top->setTextFormat(Qt::PlainText);
        main_layout->addWidget(status_top);

        status_bottom = new QLabel(main_content);
        status_bottom->setTextFormat(Qt::PlainText);
        main_layout->addWidget(status_bottom);

        watcher_table = new QTableWidget(main_content);
        watcher_table->setColumnCount(6);
        watcher_table->setHorizontalHeaderLabels({QStringLiteral("subject"),
                                                  QStringLiteral("files status"),
                                                  QStringLiteral("checking status"),
                                                  QStringLiteral("status"),
                                                  QStringLiteral("items"),
                                                  QStringLiteral("added")});
        watcher_table->verticalHeader()->setVisible(false);
        watcher_table->setSelectionBehavior(QAbstractItemView::SelectRows);
        watcher_table->setSelectionMode(QAbstractItemView::SingleSelection);
        watcher_table->setEditTriggers(QAbstractItemView::NoEditTriggers);
        watcher_table->setHorizontalScrollMode(QAbstractItemView::ScrollPerPixel);
        watcher_table->setFixedHeight(112);
        watcher_table->setTextElideMode(Qt::ElideRight);
        watcher_table->horizontalHeader()->setTextElideMode(Qt::ElideRight);
        watcher_table->horizontalHeader()->setDefaultAlignment(Qt::AlignLeft | Qt::AlignVCenter);
        watcher_table->horizontalHeader()->setFixedHeight(24);

        const int watcher_widths[] = {80, 38, 38, 48, 38, 46};

        for (int index = 0; index < 6; ++index) {
            watcher_table->setColumnWidth(index, watcher_widths[index]);
            watcher_table->horizontalHeader()->setSectionResizeMode(index, QHeaderView::Fixed);
        }

        main_layout->addWidget(watcher_table);

        auto *watcher_icons = new QWidget(main_content);
        auto *watcher_icons_layout = new QHBoxLayout(watcher_icons);
        watcher_icons_layout->setContentsMargins(0, 0, 0, 0);
        watcher_icons_layout->setSpacing(2);
        watcher_icons_layout->addStretch(1);

        watcher_highlight =
            make_row_icon(watcher_icons, QStringLiteral("highlight"), QStringLiteral("highlight"));
        watcher_icons_layout->addWidget(watcher_highlight, 0);
        watcher_clear = make_row_icon(
            watcher_icons, QStringLiteral("unhighlight"), QStringLiteral("clear highlight"));
        watcher_icons_layout->addWidget(watcher_clear, 0);
        watcher_files_pause = make_row_icon(
            watcher_icons, QStringLiteral("pause"), QStringLiteral("pause/play files"));
        watcher_icons_layout->addWidget(watcher_files_pause, 0);
        watcher_check_pause = make_row_icon(
            watcher_icons, QStringLiteral("pause"), QStringLiteral("pause/play checking"));
        watcher_icons_layout->addWidget(watcher_check_pause, 0);

        auto *check_now = new QPushButton(QStringLiteral("check now"), watcher_icons);
        check_now->setMinimumHeight(22);
        watcher_icons_layout->addWidget(check_now, 0);

        watcher_retry =
            make_row_icon(watcher_icons, QStringLiteral("retry"), QStringLiteral("retry commands"));
        watcher_icons_layout->addWidget(watcher_retry, 0);
        watcher_trash = make_row_icon(
            watcher_icons, QStringLiteral("trash"), QStringLiteral("remove selected"));
        watcher_icons_layout->addWidget(watcher_trash, 0);
        main_layout->addWidget(watcher_icons);

        auto *watcher_url_row = new QWidget(main_content);
        auto *watcher_url_layout = new QHBoxLayout(watcher_url_row);
        watcher_url_layout->setContentsMargins(0, 0, 0, 0);
        watcher_url_layout->setSpacing(2);

        watcher_url = new QLineEdit(watcher_url_row);
        watcher_url->setPlaceholderText(QStringLiteral("watcher url"));
        watcher_url_layout->addWidget(watcher_url, 1);
        watcher_paste = make_row_icon(
            watcher_url_row,
            QStringLiteral("paste"),
            QStringLiteral("Paste multiple inputs from the clipboard. Assumes the texts are "
                           "newline-separated."));
        watcher_url_layout->addWidget(watcher_paste, 0);
        main_layout->addWidget(watcher_url_row);

        checker_options = new QPushButton(QStringLiteral("checker options"), main_content);
        checker_options->setMinimumHeight(22);
        main_layout->addWidget(checker_options);

        connect(check_now, &QPushButton::clicked, this, [this]() {
            if (!main->building) {
                start();
            }
        });
        connect(watcher_highlight, &QToolButton::clicked, this, [this]() {
            if (watcher_table->rowCount() > 0) {
                watcher_table->selectRow(0);
            }
        });
        connect(watcher_clear, &QToolButton::clicked, this, [this]() {
            watcher_table->clearSelection();
        });
        connect(watcher_files_pause, &QToolButton::clicked, this, [this]() {
            main->notify(QStringLiteral("downloader_pause"), id);
        });
        connect(watcher_check_pause, &QToolButton::clicked, this, [this]() {
            main->notify(QStringLiteral("downloader_pause"), id);
        });
        connect(watcher_retry, &QToolButton::clicked, this, [this]() {
            auto *menu = new QMenu(this);
            menu->setAttribute(Qt::WA_DeleteOnClose);
            menu->addAction(QStringLiteral("retry failed"), this, [this]() {
                main->notify(QStringLiteral("downloader_retry"), id);
            });
            menu->popup(
                watcher_retry->mapToGlobal(QPoint(0, watcher_retry->height())));
        });
        connect(watcher_trash, &QToolButton::clicked, this, [this]() {
            main->notify(QStringLiteral("watcher_remove"), id);
        });
        connect(watcher_url, &QLineEdit::returnPressed, this, [this]() {
            const QString text = watcher_url->text().trimmed();

            if (!text.isEmpty()) {
                main->notify(QStringLiteral("watcher_add_url"), id, text);
                watcher_url->clear();
            }
        });
        connect(watcher_paste, &QToolButton::clicked, this, [this]() {
            const QString text = QApplication::clipboard()->text().trimmed();

            if (!text.isEmpty()) {
                const QStringList lines = text.split(QRegularExpression(QStringLiteral("[\r\n]+")),
                                                     Qt::SkipEmptyParts);

                if (!lines.isEmpty()) {
                    main->notify(QStringLiteral("watcher_add_url"), id, lines.first().trimmed());
                }
            }
        });
        connect(checker_options, &QPushButton::clicked, this, [this]() {
            main->notify(QStringLiteral("watcher_checker_options"), id);
        });
    }
}

void HydrusDownloaderPage::build_highlight_panel(QVBoxLayout *sidebar_layout, const QJsonObject &state) {
    const bool gallery = kind == QStringLiteral("gallery");
    const bool watcher = kind == QStringLiteral("watcher");
    if (gallery || watcher) {
        QVBoxLayout *highlight_layout = nullptr;
        QPushButton *highlight_expand = nullptr;
        highlight_box = make_static_box(
            gallery ? QStringLiteral("highlighted query")
                    : QStringLiteral("highlighted watcher"),
            sidebar_content,
            &highlight_layout,
            &highlight_expand);
        highlight_content = highlight_layout->parentWidget();
        sidebar_layout->addWidget(highlight_box);

        connect(highlight_expand, &QPushButton::clicked, this, [this, highlight_expand]() {
            const bool show = !highlight_content->isVisible();
            highlight_content->setVisible(show);
            highlight_expand->setIcon(
                style()->standardIcon(show ? QStyle::SP_ArrowUp : QStyle::SP_ArrowDown));
        });

        highlight_query = new QLineEdit(highlight_content);
        highlight_query->setReadOnly(true);
        highlight_query->setFixedHeight(22);

        if (watcher) {
            highlight_subject = new QLabel(highlight_content);
            highlight_subject->setTextFormat(Qt::PlainText);
            QPalette subject_palette = highlight_subject->palette();
            subject_palette.setColor(QPalette::WindowText, QColor(110, 110, 110));
            highlight_subject->setPalette(subject_palette);
            highlight_layout->addWidget(highlight_subject);
        }

        highlight_layout->addWidget(highlight_query);

        imports_box = make_status_box(highlight_content,
                                      highlight_layout,
                                      QStringLiteral("imports"),
                                      &imports_status,
                                      &file_seed_status,
                                      &files_pause,
                                      &file_log_button,
                                      &file_progress,
                                      &file_percent,
                                      &file_network_progress,
                                      &file_network_percent,
                                      &file_cog,
                                      QStringLiteral("pause/play files"),
                                      QStringLiteral("file"),
                                      QStringLiteral("file_seed_cache"));

        search_box = make_status_box(highlight_content,
                                     highlight_layout,
                                     QStringLiteral("search"),
                                     &search_status,
                                     &search_seed_status,
                                     &search_pause,
                                     &search_log_button,
                                     &search_progress,
                                     &search_percent,
                                     &search_network_progress,
                                     &search_network_percent,
                                     &search_cog,
                                     QStringLiteral("pause/play search"),
                                     gallery ? QStringLiteral("search")
                                             : QStringLiteral("check"),
                                     QStringLiteral("gallery_seed_log"));

        if (gallery) {
            highlight_limit_row = new QWidget(highlight_content);
        auto *highlight_limit_layout = new QHBoxLayout(highlight_limit_row);
        highlight_limit_layout->setContentsMargins(0, 0, 0, 0);
        highlight_limit_layout->setSpacing(4);
        highlight_limit_layout->addWidget(
            new QLabel(QStringLiteral("stop after this many files:"), highlight_limit_row), 0);

        highlight_file_limit = new QSpinBox(highlight_limit_row);
        highlight_file_limit->setRange(1, 10000000);
        highlight_file_limit->setValue(2000);
        highlight_file_limit->setFixedHeight(20);
        highlight_limit_layout->addWidget(highlight_file_limit, 1);

        highlight_no_limit = new QCheckBox(QStringLiteral("no limit"), highlight_limit_row);
        highlight_limit_layout->addWidget(highlight_no_limit, 0);
        highlight_layout->addWidget(highlight_limit_row);

        connect(
            highlight_file_limit,
            QOverload<int>::of(&QSpinBox::valueChanged),
            this,
            [this](int value) { apply_file_limit(value, highlight_no_limit->isChecked()); });
        connect(highlight_no_limit, &QCheckBox::toggled, this, [this](bool checked) {
            apply_file_limit(highlight_file_limit->value(), checked);
        });

        highlight_import_options = new QPushButton(
            QStringLiteral("import options (%1)")
                .arg(state.value(QStringLiteral("import_options_summary")).toString()),
            highlight_content);
        highlight_import_options->setMinimumHeight(22);
        highlight_layout->addWidget(highlight_import_options);

        connect(highlight_import_options, &QPushButton::clicked, this, [this]() {
            main->notify(QStringLiteral("downloader_import_options"), id);
        });
        }
    }
}