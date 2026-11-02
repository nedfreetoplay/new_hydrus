
// WS-13.9: `database -> view file history` dialog, mirroring
// `ReviewFileHistory`. Left side is a simplified search panel (the port fixes
// the domain to all known files); right side holds the status line, the
// QPainter chart, the four `show ...` checkboxes, `min y`/`max y` plus
// `refit y axis`, and two date edits plus `refit x axis`. Range maths live in
// Rust (`hydrus_file_history_auto_range`); checkbox/date/spin behaviour
// follows the Python `_Flip*`/`_Update*`/`_Auto*` methods.

#include <hydrus_client/src/gui/qt/api.cxx.h>
#include "support/support.hpp"

#include <QCheckBox>
#include <QDateEdit>
#include <QDateTime>
#include <QHBoxLayout>
#include <QLabel>
#include <QLineEdit>
#include <QPointer>
#include <QPushButton>
#include <QSpinBox>
#include <QTime>
#include <QVBoxLayout>

namespace {

QPointer<HydrusFileHistoryDialog> g_file_history_dialog;

qint64 date_to_secs(const QDate &date) {
    return QDateTime(date, QTime(0, 0, 0)).toSecsSinceEpoch();
}

QDate secs_to_date(qint64 secs) {
    return QDateTime::fromSecsSinceEpoch(secs).date();
}

} // namespace

HydrusFileHistoryDialog::HydrusFileHistoryDialog(
    HydrusMainWindow *main_window, const FileHistoryDialogState &state, QWidget *parent)
    : QDialog(parent) {
    setAttribute(Qt::WA_DeleteOnClose);
    resize(1080, 640);

    auto *hbox = new QHBoxLayout(this);

    // Simplified left search panel: same place as the Python autocomplete, but
    // the domain is fixed to all known files in this build.
    auto *search_panel = new QWidget(this);
    search_panel->setFixedWidth(260);
    auto *search_vbox = new QVBoxLayout(search_panel);
    auto *search_title = new QLabel(QStringLiteral("search"), search_panel);
    search_vbox->addWidget(search_title);
    auto *search_box = new QLineEdit(search_panel);
    search_box->setEnabled(false);
    search_box->setPlaceholderText(QStringLiteral("file search is not available in this build"));
    search_vbox->addWidget(search_box);
    auto *search_note = new QLabel(
        QStringLiteral("The file search context is fixed to all known files in this build."),
        search_panel);
    search_note->setWordWrap(true);
    search_vbox->addWidget(search_note);
    search_vbox->addStretch(1);
    hbox->addWidget(search_panel);

    auto *right = new QWidget(this);
    auto *right_vbox = new QVBoxLayout(right);

    status_ = new QLabel(right);
    right_vbox->addWidget(status_);

    chart_panel_ = new QWidget(right);
    auto *chart_vbox = new QVBoxLayout(chart_panel_);
    chart_vbox->setContentsMargins(0, 0, 0, 0);

    auto *checks = new QHBoxLayout();
    show_all_ = new QCheckBox(QStringLiteral("show all"), chart_panel_);
    show_inbox_ = new QCheckBox(QStringLiteral("show inbox"), chart_panel_);
    show_archive_ = new QCheckBox(QStringLiteral("show archive"), chart_panel_);
    show_deleted_ = new QCheckBox(QStringLiteral("show deleted"), chart_panel_);
    checks->addWidget(show_all_);
    checks->addWidget(show_inbox_);
    checks->addWidget(show_archive_);
    checks->addWidget(show_deleted_);
    chart_vbox->addLayout(checks);

    chart_ = new HydrusFileHistoryChart(chart_panel_);
    chart_vbox->addWidget(chart_, 1);

    auto *y_row = new QHBoxLayout();
    min_y_ = new QSpinBox(chart_panel_);
    min_y_->setRange(0, 1000000000);
    max_y_ = new QSpinBox(chart_panel_);
    max_y_->setRange(0, 1000000000);
    auto *refit_y = new QPushButton(QStringLiteral("refit y axis"), chart_panel_);
    refit_y->setToolTip(QStringLiteral(
        "Recalculate a y axis range based on the current data that is in view."));
    y_row->addWidget(min_y_, 1);
    y_row->addWidget(max_y_, 1);
    y_row->addWidget(refit_y);
    chart_vbox->addLayout(y_row);

    auto *x_row = new QHBoxLayout();
    start_date_ = new QDateEdit(chart_panel_);
    end_date_ = new QDateEdit(chart_panel_);
    auto *refit_x = new QPushButton(QStringLiteral("refit x axis"), chart_panel_);
    refit_x->setToolTip(QStringLiteral(
        "Recalculate an x axis range based on the current data that is in view."));
    x_row->addWidget(start_date_, 1);
    x_row->addWidget(end_date_, 1);
    x_row->addWidget(refit_x);
    chart_vbox->addLayout(x_row);

    right_vbox->addWidget(chart_panel_, 1);
    hbox->addWidget(right, 1);

    connect(show_all_, &QCheckBox::toggled, this, [this](bool) { applyCheckboxes(); });
    connect(show_inbox_, &QCheckBox::toggled, this, [this](bool) { applyCheckboxes(); });
    connect(show_archive_, &QCheckBox::toggled, this, [this](bool) { applyCheckboxes(); });
    connect(show_deleted_, &QCheckBox::toggled, this, [this](bool) { applyCheckboxes(); });
    connect(min_y_, QOverload<int>::of(&QSpinBox::valueChanged), this, [this](int) { updateYRange(); });
    connect(max_y_, QOverload<int>::of(&QSpinBox::valueChanged), this, [this](int) { updateYRange(); });
    connect(start_date_, &QDateEdit::dateChanged, this, [this](const QDate &) { updateXRange(); });
    connect(end_date_, &QDateEdit::dateChanged, this, [this](const QDate &) { updateXRange(); });
    connect(refit_y, &QPushButton::clicked, this, [this]() { autoYRange(); });
    connect(refit_x, &QPushButton::clicked, this, [this]() { autoXRange(); });

    Q_UNUSED(main_window);
    refresh(state);
}

void HydrusFileHistoryDialog::refresh(const FileHistoryDialogState &state) {
    setWindowTitle(to_qstring(state.title));

    series_.clear();
    for (const FileHistorySeries &series : state.series) {
        HydrusFileHistoryChart::DrawSeries draw;
        draw.name = to_qstring(series.name);
        draw.color = QColor(to_qstring(series.color));
        draw.visible = series.visible;
        for (const FileHistoryPoint &point : series.points) {
            draw.points.append(QPointF(double(point.timestamp), double(point.count)));
        }
        series_.append(draw);
    }

    user_custom_x_ = false;
    user_custom_y_ = false;

    show_all_->blockSignals(true);
    show_inbox_->blockSignals(true);
    show_archive_->blockSignals(true);
    show_deleted_->blockSignals(true);
    min_y_->blockSignals(true);
    max_y_->blockSignals(true);
    start_date_->blockSignals(true);
    end_date_->blockSignals(true);

    show_all_->setChecked(state.show_all);
    show_inbox_->setChecked(state.show_inbox);
    show_archive_->setChecked(state.show_archive);
    show_deleted_->setChecked(state.show_deleted);
    min_y_->setValue(static_cast<int>(state.min_y));
    max_y_->setValue(static_cast<int>(state.max_y));
    if (state.has_range) {
        start_date_->setDate(secs_to_date(state.start_date));
        end_date_->setDate(secs_to_date(state.end_date));
    }

    show_all_->blockSignals(false);
    show_inbox_->blockSignals(false);
    show_archive_->blockSignals(false);
    show_deleted_->blockSignals(false);
    min_y_->blockSignals(false);
    max_y_->blockSignals(false);
    start_date_->blockSignals(false);
    end_date_->blockSignals(false);

    const bool empty = state.empty;
    status_->setVisible(empty);
    chart_panel_->setVisible(!empty);
    if (empty) {
        status_->setText(to_qstring(state.status));
        return;
    }

    chart_->setSeries(series_);
    chart_->setYRange(state.min_y, state.max_y);
    if (state.has_range) {
        chart_->setXRange(state.start_date, state.end_date);
    }
}

rust::Vec<FileHistoryPoint> HydrusFileHistoryDialog::rustPoints(int index) const {
    rust::Vec<FileHistoryPoint> out;
    if (index < 0 || index >= series_.size()) {
        return out;
    }

    out.reserve(size_t(series_[index].points.size()));
    for (const QPointF &point : series_[index].points) {
        FileHistoryPoint row;
        row.timestamp = int64_t(point.x());
        row.count = int64_t(point.y());
        out.push_back(row);
    }
    return out;
}

void HydrusFileHistoryDialog::applyCheckboxes() {
    if (series_.size() >= 4) {
        series_[0].visible = show_all_->isChecked();
        series_[1].visible = show_inbox_->isChecked();
        series_[2].visible = show_archive_->isChecked();
        series_[3].visible = show_deleted_->isChecked();
        chart_->setSeries(series_);
    }

    if (!user_custom_y_) {
        autoYRange();
    }
}

void HydrusFileHistoryDialog::autoYRange() {
    user_custom_y_ = false;

    const FileHistoryRange range = hydrus_file_history_auto_range(
        rustPoints(0), rustPoints(1), rustPoints(2), rustPoints(3), show_all_->isChecked(),
        show_inbox_->isChecked(), show_archive_->isChecked(), show_deleted_->isChecked());

    min_y_->blockSignals(true);
    max_y_->blockSignals(true);
    min_y_->setValue(static_cast<int>(range.min_y));
    max_y_->setValue(static_cast<int>(range.max_y));
    min_y_->blockSignals(false);
    max_y_->blockSignals(false);

    chart_->setYRange(range.min_y, range.max_y);
}

void HydrusFileHistoryDialog::autoXRange() {
    user_custom_x_ = false;

    const FileHistoryRange range = hydrus_file_history_auto_range(
        rustPoints(0), rustPoints(1), rustPoints(2), rustPoints(3), show_all_->isChecked(),
        show_inbox_->isChecked(), show_archive_->isChecked(), show_deleted_->isChecked());

    if (!range.has_range) {
        return;
    }

    start_date_->blockSignals(true);
    end_date_->blockSignals(true);
    start_date_->setDate(secs_to_date(range.start));
    end_date_->setDate(secs_to_date(range.end));
    start_date_->blockSignals(false);
    end_date_->blockSignals(false);

    chart_->setXRange(range.start, range.end);
}

void HydrusFileHistoryDialog::updateYRange() {
    user_custom_y_ = true;
    chart_->setYRange(min_y_->value(), max_y_->value());
}

void HydrusFileHistoryDialog::updateXRange() {
    user_custom_x_ = true;
    chart_->setXRange(date_to_secs(start_date_->date()), date_to_secs(end_date_->date()));
}

void hydrus_show_file_history(HydrusMainWindow &window, const FileHistoryDialogState &state) {
    if (g_file_history_dialog) {
        g_file_history_dialog->refresh(state);
        g_file_history_dialog->show();
        g_file_history_dialog->raise();
        g_file_history_dialog->activateWindow();
        return;
    }

    auto *dialog = new HydrusFileHistoryDialog(&window, state, &window);
    g_file_history_dialog = dialog;
    dialog->show();
    dialog->raise();
    dialog->activateWindow();
}
